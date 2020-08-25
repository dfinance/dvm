use libra::prelude::*;

use std::convert::TryInto;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::Error;
use api::grpc::ds_grpc::{ds_raw_response::ErrorCode, ds_service_client::DsServiceClient, DsAccessPath};
use crossbeam::channel::{bounded, Receiver, Sender};
use http::Uri;
use tokio::runtime::Runtime;
use dvm_net::api;
use dvm_net::prelude::*;
use dvm_net::tonic;

use crate::{Clear, DataSource};

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
                            }))
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
                        let grpc_request = tonic::Request::new(access_path_into_ds(request.path));
                        let res = client.get_raw(grpc_request).await;
                        if let Err(ref err) = res {
                            error!(
                                "Transport-level error received by data source ({:?}). {}",
                                std::thread::current(),
                                err
                            );
                            std::thread::sleep(Duration::from_millis(500));
                            std::process::exit(-1);
                        }
                        let response = res.unwrap().into_inner();
                        let error_code = ErrorCode::from_i32(response.error_code)
                            .expect("Invalid ErrorCode enum value");

                        let response = match error_code {
                            // if no error code, return blob
                            ErrorCode::None => Ok(Some(response.blob)),
                            // if BadRequest, return Err()
                            ErrorCode::BadRequest => Err(anyhow!(response.error_message)),
                            // if NoData, return None
                            ErrorCode::NoData => Ok(None),
                        };
                        if let Err(err) = request.sender.send(response) {
                            error!("Internal VM-DS channel error: {:?}", err);
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
    pub fn get(&self, access_path: &AccessPath) -> Result<Option<Vec<u8>>, Error> {
        let (tx, rx) = bounded(0);
        self.sender.send(Request {
            path: access_path.clone(),
            sender: tx,
        })?;
        rx.recv()?
    }
}

/// Convert Libra's `AccessPath` into gRPC `DsAccessPath`.
pub fn access_path_into_ds(ap: AccessPath) -> DsAccessPath {
    DsAccessPath::new(ap.address.to_vec(), ap.path)
}

struct Request {
    path: AccessPath,
    sender: Sender<Result<Option<Vec<u8>>, Error>>,
}

impl RemoteCache for GrpcDataSource {
    fn get_module(&self, module_id: &ModuleId) -> VMResult<Option<Vec<u8>>> {
        self.get(&AccessPath::from(module_id)).map_err(|e| {
            PartialVMError::new(StatusCode::STORAGE_ERROR)
                .with_message(e.to_string())
                .finish(Location::Undefined)
        })
    }

    fn get_resource(
        &self,
        address: &AccountAddress,
        tag: &TypeTag,
    ) -> PartialVMResult<Option<Vec<u8>>> {
        let struct_tag = match tag {
            TypeTag::Struct(struct_tag) => struct_tag.clone(),
            _ => return Err(PartialVMError::new(StatusCode::VALUE_DESERIALIZATION_ERROR)),
        };
        let resource_tag = ResourceKey::new(*address, struct_tag);
        let path = AccessPath::resource_access_path(&resource_tag);

        self.get(&path)
            .map_err(|e| PartialVMError::new(StatusCode::STORAGE_ERROR).with_message(e.to_string()))
    }
}

impl Clear for GrpcDataSource {}

impl DataSource for GrpcDataSource {}
