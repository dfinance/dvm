use std::thread;
use tokio::runtime::Runtime;
use dvm_net::tonic::{self, transport::Server};
use dvm_net::tonic::{Request, Response, Status};
use dvm_net::api::grpc;
use grpc::ds_grpc::ds_service_server::{DsServiceServer, DsService};
use grpc::ds_grpc::{DsAccessPath, DsRawResponse, DsAccessPaths, DsRawResponses};
use std::time::Duration;
use dvm_data_source::GrpcDataSource;
use libra::prelude::*;

const ADDRESS: &str = "127.0.0.1:8080";

struct DataSourceService {}

#[tonic::async_trait]
impl DsService for DataSourceService {
    #[allow(clippy::transmute_ptr_to_ref)]
    async fn get_raw(
        &self,
        request: Request<DsAccessPath>,
    ) -> Result<Response<DsRawResponse>, Status> {
        let mut request: DsAccessPath = request.into_inner();
        let mut response = Vec::with_capacity(request.path.len() + request.address.len());
        response.append(&mut request.address);
        response.append(&mut request.path);
        Ok(Response::new(DsRawResponse::with_blob(&response)))
    }

    async fn multi_get_raw(
        &self,
        _request: Request<DsAccessPaths>,
    ) -> Result<Response<DsRawResponses>, Status> {
        Err(Status::invalid_argument("method not implemented."))
    }
}

pub fn run_ds_service_mock() {
    thread::spawn(move || {
        let mut rt = Runtime::new().unwrap();
        rt.block_on(async {
            Server::builder()
                .add_service(DsServiceServer::new(DataSourceService {}))
                .serve(ADDRESS.parse().unwrap())
                .await
                .unwrap();
        });
    });
    thread::sleep(Duration::from_secs(1));
}

#[allow(clippy::needless_collect)]
#[test]
fn test_grpc_ds() {
    run_ds_service_mock();
    let ds = GrpcDataSource::new(
        format!("http://{}", ADDRESS).parse().unwrap(),
        Default::default(),
    )
    .unwrap();

    let handlers = (0..8)
        .map(|_| ds.clone())
        .map(|ds| {
            thread::spawn(move || {
                let mut is_ok = true;
                for _ in 0..100 {
                    let path = AccessPath::new(
                        AccountAddress::random(),
                        AccountAddress::random().to_vec(),
                    );
                    if let Ok(Some(resp)) = ds.get(&path) {
                        let mut response =
                            Vec::with_capacity(path.address.as_ref().len() + path.path.len());
                        response.append(&mut path.address.to_vec());
                        response.append(&mut path.path.to_vec());
                        if resp != response {
                            is_ok = false;
                            break;
                        }
                    } else {
                        is_ok = false;
                        break;
                    }
                }
                is_ok
            })
        })
        .collect::<Vec<_>>();

    assert!(handlers.into_iter().map(|h| h.join().unwrap()).all(|v| v));
}
