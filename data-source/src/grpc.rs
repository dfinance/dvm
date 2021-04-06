use std::convert::TryInto;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::Error;
use api::grpc::{ds_service_client::DsServiceClient, DsAccessPath};
use crossbeam::channel::{bounded, Receiver, Sender};
use http::Uri;
use tokio::runtime::Runtime;

use dvm_net::api;
use dvm_net::api::grpc::{
    CurrencyInfoRequest as GCurrencyInfoRequest, ErrorCode, NativeBalanceRequest,
    OraclePriceRequest, U128,
};
use dvm_net::prelude::*;
use dvm_net::tonic;
use dvm_net::tonic::Status;
use libra::prelude::*;

use crate::{Balance, CurrencyInfo, DataSource, GetCurrencyInfo, Oracle, RemoveModule};

/// Receiver for a channel that handles shutdown signals.
pub type ShutdownSig = tokio::sync::oneshot::Receiver<()>;

/// Wrapper around gRPC-based interface to dnode. Used for the resource resolution inside the VM.
#[derive(Clone)]
pub struct GrpcDataSource {
    handler: Arc<JoinHandle<()>>,
    sender: Sender<Request>,
}

impl GrpcDataSource {
    /// Create an instance of gRPC based data source for VM.
    /// `shutdown_signal` is a oneshot `crossbeam_channel::Sender` to shutdown the service.
    pub fn new(uri: Uri, shutdown_signal: Option<ShutdownSig>) -> Result<GrpcDataSource, Error> {
        let rt = Runtime::new()?;
        let (sender, receiver) = bounded(10);
        let handler =
            thread::spawn(move || Self::internal_loop(rt, uri, receiver, shutdown_signal));

        Ok(GrpcDataSource {
            handler: Arc::new(handler),
            sender,
        })
    }

    fn internal_loop(
        mut rt: Runtime,
        ds_addr: Uri,
        receiver: Receiver<Request>,
        mut shutdown_signal: Option<ShutdownSig>,
    ) {
        info!("Connecting to data-source: {}", ds_addr);
        let client: Option<DsServiceClient<_>> = rt.block_on(async {
            while !(&mut shutdown_signal)
                .as_mut()
                .map(|rx| rx.try_recv().is_ok())
                .unwrap_or(false)
            {
                match ds_addr.clone().try_into() {
                    Err(err) => {
                        error!("Invalid DS address: {:?}", err);
                        std::thread::sleep(Duration::from_millis(500));
                        std::process::exit(-1);
                    }
                    Ok::<Endpoint, _>(endpoint) => match endpoint.connect().await {
                        Ok(channel) => {
                            return Some(DsServiceClient::with_interceptor(channel, |req| {
                                debug!("request DS: {:?}", req);
                                Ok(req)
                            }));
                        }
                        Err(_) => tokio::time::delay_for(Duration::from_secs(1)).await,
                    },
                }
            }

            // Fallback, when while ended without return.
            // It can happen when shutdown signal is received.
            // So we should log this and return None.
            info!("DS client shutted down");
            None
        });

        // We are connected if client is Some.
        if let Some(mut client) = client {
            info!("Connected to data-source");

            rt.block_on(async {
                while !shutdown_signal
                    .as_mut()
                    .map(|rx| rx.try_recv().is_ok())
                    .unwrap_or(false)
                {
                    if let Ok(request) = receiver.recv() {
                        match request {
                            Request::StateView(StateViewRequest { request, handler }) => {
                                let resp = unwrap_error(client.get_raw(request).await)
                                    .await
                                    .into_inner();
                                handler.send(handle_response(
                                    resp.error_code,
                                    resp.error_message,
                                    Some(resp.blob),
                                ));
                            }
                            Request::Oracle(OracleRequest { request, handler }) => {
                                let resp = unwrap_error(client.get_oracle_price(request).await)
                                    .await
                                    .into_inner();
                                handler.send(handle_response(
                                    resp.error_code,
                                    resp.error_message,
                                    resp.price.map(U128::into),
                                ));
                            }
                            Request::Balance(BalanceRequest { request, handler }) => {
                                let resp = unwrap_error(client.get_native_balance(request).await)
                                    .await
                                    .into_inner();
                                handler.send(handle_response(
                                    resp.error_code,
                                    resp.error_message,
                                    resp.balance.map(U128::into),
                                ));
                            }
                            Request::CurrencyInfo(CurrencyInfoRequest { request, handler }) => {
                                let resp = unwrap_error(client.get_currency_info(request).await)
                                    .await
                                    .into_inner();
                                handler.send(handle_response(
                                    resp.error_code,
                                    resp.error_message,
                                    resp.info.and_then(|info| {
                                        Some(CurrencyInfo {
                                            denom: info.denom,
                                            decimals: info.decimals as u8,
                                            is_token: info.is_token,
                                            address: AccountAddress::try_from(info.address).ok()?,
                                            total_supply: u128::from(info.total_supply?),
                                        })
                                    }),
                                ));
                            }
                        }
                    }
                }
            });

            // We there in case of:
            // - DS connection is broken,
            // - we just received the shutdown signal.
            // Anyway, that's the finish. Just log it.
            info!("DS client shutted down");
        } else {
            // client is None, so we cannot connect and cannot continue.
            warn!("Unable to connect to data-source.");
        }
    }

    /// Returns chain data by access path.
    pub fn get_sv(&self, path: AccessPath) -> Result<Option<Vec<u8>>, Error> {
        let (tx, rx) = bounded(0);
        self.sender.send(Request::StateView(StateViewRequest {
            request: tonic::Request::new(DsAccessPath {
                address: path.address.to_vec(),
                path: path.path,
            }),
            handler: StateViewHandler(tx),
        }))?;
        rx.recv()?
    }
}

/// Convert Libra's `AccessPath` into gRPC `DsAccessPath`.
pub fn access_path_into_ds(ap: AccessPath) -> DsAccessPath {
    DsAccessPath::new(ap.address.to_vec(), ap.path)
}

enum Request {
    StateView(StateViewRequest),
    Oracle(OracleRequest),
    Balance(BalanceRequest),
    CurrencyInfo(CurrencyInfoRequest),
}

struct CurrencyInfoRequest {
    request: tonic::Request<GCurrencyInfoRequest>,
    handler: CurrencyInfoHandler,
}

struct CurrencyInfoHandler(Sender<Result<Option<CurrencyInfo>, Error>>);

struct StateViewRequest {
    request: tonic::Request<DsAccessPath>,
    handler: StateViewHandler,
}

struct StateViewHandler(Sender<Result<Option<Vec<u8>>, Error>>);

struct OracleRequest {
    request: tonic::Request<OraclePriceRequest>,
    handler: OracleHandler,
}

struct OracleHandler(Sender<Result<Option<u128>, Error>>);

struct BalanceRequest {
    request: tonic::Request<NativeBalanceRequest>,
    handler: BalanceHandler,
}

struct BalanceHandler(Sender<Result<Option<u128>, Error>>);

async fn unwrap_error<T>(res: Result<T, Status>) -> T {
    match res {
        Ok(t) => t,
        Err(err) => {
            error!(
                "Transport-level error received by data source ({:?}). {}",
                std::thread::current(),
                err
            );
            std::thread::sleep(Duration::from_millis(500));
            std::process::exit(-1);
        }
    }
}

fn handle_response<T>(code: i32, err_msg: String, msg: Option<T>) -> Result<Option<T>, Error> {
    let error_code = ErrorCode::from_i32(code).expect("Invalid ErrorCode enum value");
    match error_code {
        // if no error code, return msg
        ErrorCode::None => Ok(msg),
        // if BadRequest, return Err()
        ErrorCode::BadRequest => Err(anyhow!(err_msg)),
        // if NoData, return None
        ErrorCode::NoData => Ok(None),
    }
}

trait SendResult {
    type Resp;
    fn send(&self, response: Result<Option<Self::Resp>, Error>);
}

impl SendResult for StateViewHandler {
    type Resp = Vec<u8>;

    fn send(&self, response: Result<Option<Self::Resp>, Error>) {
        if let Err(err) = self.0.send(response) {
            error!("Internal VM-DS channel error: {:?}", err);
        }
    }
}

impl SendResult for CurrencyInfoHandler {
    type Resp = CurrencyInfo;

    fn send(&self, response: Result<Option<Self::Resp>, Error>) {
        if let Err(err) = self.0.send(response) {
            error!("Internal VM-DS channel error: {:?}", err);
        }
    }
}

impl SendResult for OracleHandler {
    type Resp = u128;

    fn send(&self, response: Result<Option<Self::Resp>, Error>) {
        if let Err(err) = self.0.send(response) {
            error!("Internal VM-DS channel error: {:?}", err);
        }
    }
}

impl SendResult for BalanceHandler {
    type Resp = u128;

    fn send(&self, response: Result<Option<Self::Resp>, Error>) {
        if let Err(err) = self.0.send(response) {
            error!("Internal VM-DS channel error: {:?}", err);
        }
    }
}

impl RemoteCache for GrpcDataSource {
    fn get_module(&self, module_id: &ModuleId) -> VMResult<Option<Vec<u8>>> {
        self.get_sv(AccessPath::from(module_id)).map_err(|e| {
            PartialVMError::new(StatusCode::STORAGE_ERROR)
                .with_message(e.to_string())
                .finish(Location::Undefined)
        })
    }

    fn get_resource(
        &self,
        address: &AccountAddress,
        tag: &StructTag,
    ) -> PartialVMResult<Option<Vec<u8>>> {
        let resource_tag = ResourceKey::new(*address, tag.to_owned());
        let path = AccessPath::resource_access_path(&resource_tag);

        self.get_sv(path)
            .map_err(|e| PartialVMError::new(StatusCode::STORAGE_ERROR).with_message(e.to_string()))
    }
}

impl RemoveModule for GrpcDataSource {}

impl Balance for GrpcDataSource {
    fn get_balance(&self, address: AccountAddress, ticker: String) -> Result<Option<u128>, Error> {
        let (tx, rx) = bounded(0);
        self.sender.send(Request::Balance(BalanceRequest {
            request: tonic::Request::new(NativeBalanceRequest {
                address: address.to_vec(),
                ticker,
            }),
            handler: BalanceHandler(tx),
        }))?;
        rx.recv()?
    }
}

impl Oracle for GrpcDataSource {
    fn get_price(&self, currency_1: String, currency_2: String) -> Result<Option<u128>, Error> {
        let (tx, rx) = bounded(0);
        self.sender.send(Request::Oracle(OracleRequest {
            request: tonic::Request::new(OraclePriceRequest {
                currency_1,
                currency_2,
            }),
            handler: OracleHandler(tx),
        }))?;
        rx.recv()?
    }
}

impl GetCurrencyInfo for GrpcDataSource {
    fn get_currency_info(&self, ticker: String) -> Result<Option<CurrencyInfo>, Error> {
        let (tx, rx) = bounded(0);
        self.sender
            .send(Request::CurrencyInfo(CurrencyInfoRequest {
                request: tonic::Request::new(GCurrencyInfoRequest { ticker }),
                handler: CurrencyInfoHandler(tx),
            }))?;
        rx.recv()?
    }
}

impl DataSource for GrpcDataSource {}
