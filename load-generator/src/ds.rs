use dvm_net::api::grpc;
use dvm_net::tonic;
use grpc::ds_grpc::ds_service_server::DsService;
use grpc::ds_grpc::{
    DsAccessPath, DsRawResponse, DsAccessPaths, DsRawResponses, ds_raw_response::ErrorCode,
};
use dvm_net::api::tonic::{Status, Response, Request};
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use anyhow::Error;
use dvm_net::api::tonic::transport::Server;
use dvm_net::api::grpc::ds_grpc::ds_service_server::DsServiceServer;
use libra::ds::{WriteSet, WriteOp};
use dvm_net::endpoint::Endpoint;
use dvm_net::serve::ServeWith;

type Resources = HashMap<Vec<u8>, Vec<u8>>;
type Accounts = HashMap<Vec<u8>, Resources>;

#[derive(Debug, Clone, Default)]
pub struct InMemoryDataSource {
    store: Arc<RwLock<Accounts>>,
}

impl InMemoryDataSource {
    pub fn new() -> InMemoryDataSource {
        InMemoryDataSource {
            store: Arc::new(Default::default()),
        }
    }

    pub fn store_write_set(&self, set: WriteSet) {
        let mut store = self.store.write().unwrap();
        for (path, wo) in set.iter() {
            let account = store.get_mut(path.address.as_ref());
            match wo {
                WriteOp::Deletion => {
                    if let Some(acc) = account {
                        acc.remove(&path.path);
                    }
                }
                WriteOp::Value(value) => {
                    if let Some(acc) = account {
                        acc.insert(path.path.to_owned(), value.to_owned());
                    } else {
                        let mut acc = HashMap::new();
                        acc.insert(path.path.to_owned(), value.to_owned());
                        store.insert(path.address.to_vec(), acc);
                    }
                }
            }
        }
    }
}

#[tonic::async_trait]
impl DsService for InMemoryDataSource {
    async fn get_raw(
        &self,
        request: Request<DsAccessPath>,
    ) -> Result<Response<DsRawResponse>, Status> {
        let path = request.get_ref();
        let store = self.store.read().unwrap();
        Ok(store
            .get(&path.address)
            .and_then(|rs| rs.get(&path.path))
            .map(|blob| Response::new(DsRawResponse::with_blob(blob)))
            .unwrap_or_else(|| {
                Response::new(DsRawResponse::with_error(
                    ErrorCode::NoData,
                    "No blob".to_owned(),
                ))
            }))
    }

    async fn multi_get_raw(
        &self,
        request: Request<DsAccessPaths>,
    ) -> Result<Response<DsRawResponses>, Status> {
        let store = self.store.read().unwrap();

        Ok(request
            .get_ref()
            .paths
            .iter()
            .map(|path| {
                store
                    .get(&path.address)
                    .and_then(|rs| rs.get(&path.path))
                    .map(|blob| blob.to_vec())
            })
            .collect::<Option<_>>()
            .map(|blobs| Response::new(DsRawResponses { blobs }))
            .unwrap_or_else(|| Response::new(DsRawResponses { blobs: vec![] })))
    }
}

pub fn start(endpoint: Endpoint) -> Result<InMemoryDataSource, Error> {
    let ds = InMemoryDataSource::new();
    let ds_clone = ds.clone();
    tokio::spawn(async move {
        println!("Start data source at {}", endpoint);
        Server::builder()
            .add_service(DsServiceServer::new(ds_clone.clone()))
            .serve_ext(endpoint)
            .await
            .unwrap()
    });
    Ok(ds)
}
