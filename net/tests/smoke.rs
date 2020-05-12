extern crate dvm_net;

use std::time::Duration;
use std::thread::JoinHandle;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use dvm_net::{api, tonic};
use dvm_net::prelude::*;

use api::grpc::ds_grpc::{
    ds_service_client::DsServiceClient,
    ds_service_server::{DsServiceServer, DsService},
    DsAccessPath, DsRawResponse, DsAccessPaths, DsRawResponses,
};

use tonic::Request;
use tonic::{transport::Server, Response, Status};
use tokio::runtime::Builder;

#[derive(Default)]
pub struct Fake();

#[tonic::async_trait]
impl DsService for Fake {
    async fn get_raw(&self, _: Request<DsAccessPath>) -> Result<Response<DsRawResponse>, Status> {
        let reply = DsRawResponse::default();
        Ok(Response::new(reply))
    }

    async fn multi_get_raw(
        &self,
        _: Request<DsAccessPaths>,
    ) -> Result<Response<DsRawResponses>, Status> {
        unimplemented!()
    }
}

fn serve(uri: &str, counter: Arc<AtomicUsize>) -> JoinHandle<()> {
    let uri = uri.to_owned();
    std::thread::spawn(move || {
        let mut rt = Builder::new()
            .basic_scheduler()
            .enable_all()
            .build()
            .unwrap();
        let endpoint: Endpoint = uri.parse().unwrap();

        let service = Fake();
        rt.block_on(async {
            Server::builder()
                .add_service(DsServiceServer::with_interceptor(service, move |req| {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok(req)
                }))
                .serve_ext(endpoint)
                .await
                .unwrap();
        });
    })
}

const CLIENT_REQS: usize = 3;
fn client(uri: &str) -> JoinHandle<()> {
    let uri = uri.to_owned();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(1));
        let mut rt = Builder::new()
            .basic_scheduler()
            .enable_all()
            .build()
            .unwrap();
        let endpoint: Endpoint = uri.parse().unwrap();

        rt.block_on(async {
            let channel = endpoint.connect().await.unwrap();
            let mut client = DsServiceClient::new(channel);
            for _ in 0..CLIENT_REQS {
                let request = tonic::Request::new(DsAccessPath {
                    address: vec![0],
                    path: vec![0],
                });
                let _: DsRawResponse = client.get_raw(request).await.unwrap().into_inner();
            }
        });
    })
}

mod http {
    use super::*;

    #[test]
    fn http_1_to_1() {
        const URI: &str = "http://[::1]:50051";
        let counter = Arc::new(AtomicUsize::new(0));
        {
            let _ = serve(URI, counter.clone());
            client(URI).join().unwrap();
        }
        assert_eq!(/* 1 *  */ CLIENT_REQS, counter.load(Ordering::Acquire));
    }

    #[test]
    fn http_1_to_many() {
        const URI: &str = "http://[::1]:50052";
        let counter = Arc::new(AtomicUsize::new(0));
        {
            // spawn server:
            let _ = serve(URI, counter.clone());
            // spawn 5 clients:
            for _ in 0..4 {
                self::client(URI);
            }
            // wait last client
            client(URI).join().unwrap();

            // wait one sec for drop:
            std::thread::sleep(Duration::from_secs(1))
        }
        assert_eq!(5 * CLIENT_REQS, counter.load(Ordering::Acquire));
    }
}

mod ipc {
    use super::*;

    #[test]
    fn ipc_1_to_1() {
        const URI: &str = "ipc://./tmp/test1.ipc";
        const PATH: &str = "./tmp/test1.ipc";
        let counter = Arc::new(AtomicUsize::new(0));
        {
            let _ = serve(URI, counter.clone());
            client(URI).join().unwrap();
        }

        #[cfg(unix)]
        let _ = dvm_net::transport::close_uds(PATH);

        assert_eq!(/* 1 *  */ CLIENT_REQS, counter.load(Ordering::Acquire));
    }

    #[test]
    fn ipc_1_to_many() {
        const URI: &str = "ipc://./tmp/test2.ipc";
        const PATH: &str = "./tmp/test2.ipc";
        let counter = Arc::new(AtomicUsize::new(0));
        {
            // spawn server
            let _ = serve(URI, counter.clone());
            // spawn 5 clients
            for _ in 0..4 {
                client(URI);
            }
            // wait last client
            client(URI).join().unwrap();

            // wait one sec for drop
            std::thread::sleep(Duration::from_secs(1))
        }

        #[cfg(unix)]
        let _ = dvm_net::transport::close_uds(PATH);

        assert_eq!(5 * CLIENT_REQS, counter.load(Ordering::Acquire));
    }
}
