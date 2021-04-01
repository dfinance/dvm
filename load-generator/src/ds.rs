use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use anyhow::Error;
use grpc::ds_grpc::{DsAccessPath, DsAccessPaths, DsRawResponse, DsRawResponses};
use grpc::ds_grpc::ds_service_server::DsService;

use dvm_net::api::grpc;
use dvm_net::api::grpc::ds_grpc::{
    ErrorCode, NativeBalanceRequest, NativeBalanceResponse, OraclePriceRequest,
    OraclePriceResponse, CurrencyInfo,
};
use dvm_net::api::grpc::ds_grpc::{CurrencyInfoRequest, CurrencyInfoResponse};
use dvm_net::api::grpc::ds_grpc::ds_service_server::DsServiceServer;
use dvm_net::api::tonic::{Request, Response, Status};
use dvm_net::api::tonic::transport::Server;
use dvm_net::endpoint::Endpoint;
use dvm_net::serve::ServeWith;
use dvm_net::tonic;
use libra::ds::{WriteOp, WriteSet};

use crate::ds::grpc::types::U128;

type Resources = HashMap<Vec<u8>, Vec<u8>>;
type Accounts = HashMap<Vec<u8>, Resources>;
type Oracles = HashMap<String, u128>;
type Balances = HashMap<(Vec<u8>, String), u128>;
type CurInfo = HashMap<String, CurrencyInfo>;

#[derive(Debug, Clone, Default)]
pub struct InMemoryDataSource {
    store: Arc<RwLock<Accounts>>,
    oracle: Arc<RwLock<Oracles>>,
    balance: Arc<RwLock<Balances>>,
    currency_info: Arc<RwLock<CurInfo>>,
}

impl InMemoryDataSource {
    pub fn new() -> InMemoryDataSource {
        InMemoryDataSource {
            store: Arc::new(Default::default()),
            oracle: Arc::new(Default::default()),
            balance: Arc::new(Default::default()),
            currency_info: Arc::new(Default::default()),
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

    async fn get_oracle_price(
        &self,
        request: Request<OraclePriceRequest>,
    ) -> Result<Response<OraclePriceResponse>, Status> {
        let store = self.oracle.read().unwrap();
        let req = request.into_inner();
        let ticker = format!("{}_{}", req.currency_1, req.currency_2);
        let resp = match store.get(&ticker) {
            None => OraclePriceResponse {
                price: None,
                error_code: ErrorCode::NoData as i32,
                error_message: "No blob".to_owned(),
            },
            Some(price) => OraclePriceResponse {
                price: Some(U128::from(*price)),
                error_code: ErrorCode::None as i32,
                error_message: "".to_string(),
            },
        };

        Ok(Response::new(resp))
    }

    async fn get_native_balance(
        &self,
        request: Request<NativeBalanceRequest>,
    ) -> Result<Response<NativeBalanceResponse>, Status> {
        let store = self.balance.read().unwrap();
        let req = request.into_inner();

        let result = match store.get(&(req.address, req.ticker)) {
            None => NativeBalanceResponse {
                balance: None,
                error_code: ErrorCode::NoData as i32,
                error_message: "No blob".to_owned(),
            },
            Some(val) => NativeBalanceResponse {
                balance: Some(U128::from(*val)),
                error_code: ErrorCode::None as i32,
                error_message: "".to_owned(),
            },
        };

        Ok(Response::new(result))
    }

    async fn get_currency_info(
        &self,
        request: Request<CurrencyInfoRequest>,
    ) -> Result<Response<CurrencyInfoResponse>, Status> {
        let store = self.currency_info.read().unwrap();
        let req = request.into_inner();
        let result = match store.get(&req.ticker) {
            None => CurrencyInfoResponse {
                info: None,
                error_code: ErrorCode::NoData as i32,
                error_message: "No blob".to_owned(),
            },
            Some(info) => CurrencyInfoResponse {
                info: Some(info.clone()),
                error_code: ErrorCode::None as i32,
                error_message: "".to_string(),
            },
        };
        Ok(Response::new(result))
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
