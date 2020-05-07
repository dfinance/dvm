extern crate dvm_net;

use std::time::Duration;
use std::thread::JoinHandle;
use dvm_net::grpc;
use dvm_net::prelude::*;
use grpc::grpc::ds_grpc::{
    ds_service_client::DsServiceClient,
    DsAccessPath, DsRawResponse, DsAccessPaths, DsRawResponses,
    ds_service_server::{DsServiceServer, DsService},
};
use dvm_net::tonic;
use tonic::Request;
use tonic::{transport::Server, Response, Status};
use tokio::runtime::Builder;

#[derive(Default)]
pub struct Fake {
    // pub counter: AtomicUsize,
}

#[tonic::async_trait]
impl DsService for Fake {
    async fn get_raw(
        &self,
        _: Request<DsAccessPath>,
    ) -> Result<Response<DsRawResponse>, Status> {
        // self.counter.fetch_add(1, Ordering::SeqCst);

        let reply = DsRawResponse {
            // blob: vec![self.counter.load(Ordering::Relaxed) as u8],
            ..Default::default()
        };
        Ok(Response::new(reply))
    }

    async fn multi_get_raw(
        &self,
        _: Request<DsAccessPaths>,
    ) -> Result<Response<DsRawResponses>, Status> {
        unimplemented!()
    }
}

fn serve(uri: &str) -> JoinHandle<()> {
    let uri = uri.to_owned();
    std::thread::spawn(move || {
        let mut rt = Builder::new()
            .basic_scheduler()
            .enable_all()
            .build()
            .unwrap();
        let endpoint: Endpoint = uri.parse().unwrap();

        let service = Fake::default();
        rt.block_on(async {
            Server::builder()
                .add_service(DsServiceServer::new(service))
                .serve_with_anyway(endpoint)
                .await
                .unwrap();
        });
    })
}

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

            for _ in 0..3 {
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
    fn test_http_1_to_1() {
        const URI: &str = "http://[::1]:50051";

        {
            let _serve = serve(URI);
            let client = client(URI);

            client.join().unwrap();
        }
    }

    #[test]
    fn test_http_1_to_many() {
        const URI: &str = "http://[::1]:50052";

        {
            let _serve = serve(URI);

            for _ in 0..4 {
                client(URI);
            }
            client(URI).join().unwrap();
        }
    }
}

mod ipc {
    use super::*;

    #[test]
    fn test_ipc_1_to_1() {
        const URI: &str = "ipc://./tmp/test1.ipc";
        const PATH: &str = "./tmp/test1.ipc";

        {
            let _serve = serve(URI);
            let client = client(URI);

            client.join().unwrap();
        }

        #[cfg(unix)]
        let _ = dvm_net::transport::unlink_uds(PATH);
    }

    #[test]
    fn test_ipc_1_to_many() {
        const URI: &str = "ipc://./tmp/test2.ipc";
        const PATH: &str = "./tmp/test2.ipc";

        {
            let _serve = serve(URI);

            for _ in 0..4 {
                client(URI);
            }
            client(URI).join().unwrap();
        }

        #[cfg(unix)]
        let _ = dvm_net::transport::unlink_uds(PATH);
    }
}
