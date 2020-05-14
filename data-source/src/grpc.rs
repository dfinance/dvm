use std::convert::TryInto;
use std::thread::{self, JoinHandle};
use std::sync::Arc;
use std::time::Duration;
use libra::{libra_types, libra_state_view};
use libra_state_view::StateView;
use libra_types::access_path::AccessPath;
use anyhow::Error;
use http::Uri;
use tokio::runtime::Runtime;
use crossbeam::channel::{Sender, Receiver, bounded};
use dvm_net::api;
use dvm_net::tonic;
use dvm_net::prelude::*;
use api::grpc::ds_grpc::{ds_service_client::DsServiceClient, DsAccessPath, ds_raw_response::ErrorCode};
use libra::move_vm_state::data_cache::RemoteCache;
use libra::libra_vm::errors::VMResult;
use libra_types::vm_error::{VMStatus, StatusCode};
use crate::{DataSource, Clear};

#[derive(Clone)]
pub struct GrpcDataSource {
    handler: Arc<JoinHandle<()>>,
    sender: Sender<Request>,
}

impl GrpcDataSource {
    pub fn new(uri: Uri) -> Result<GrpcDataSource, Error> {
        let rt = Runtime::new()?;
        let (sender, receiver) = bounded(10);
        let handler = thread::spawn(move || Self::internal_loop(rt, uri, receiver));

        Ok(GrpcDataSource {
            handler: Arc::new(handler),
            sender,
        })
    }

    fn internal_loop(mut rt: Runtime, ds_addr: Uri, receiver: Receiver<Request>) {
        info!("Connecting to data-source: {}", ds_addr);
        let mut client: DsServiceClient<_> = rt.block_on(async {
            let scheme_http = ds_addr.scheme_str().expect("scheme").starts_with("http");
            loop {
                // hotfix http://domain-not-socket/issue
                // TODO: write it normally
                if scheme_http {
                    match DsServiceClient::connect(ds_addr.to_string()).await {
                        Ok(client) => return client,
                        Err(err) => tokio::time::delay_for(Duration::from_secs(1)).await,
                    }
                } else {
                    match ds_addr.clone().try_into() {
                        Err(err) => {
                            error!("Invalid DS address: {:?}", err);
                            std::thread::sleep(Duration::from_millis(500));
                            std::process::exit(-1);
                        }
                        Ok::<Endpoint, _>(endpoint) => match endpoint.connect().await {
                            Ok(channel) => return DsServiceClient::new(channel),
                            Err(_) => tokio::time::delay_for(Duration::from_secs(1)).await,
                        },
                    }
                }
            }
        });

        info!("Connected to data-source");
        rt.block_on(async {
            loop {
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
    }
}

impl StateView for GrpcDataSource {
    fn get(&self, access_path: &AccessPath) -> Result<Option<Vec<u8>>, Error> {
        let (tx, rx) = bounded(0);
        self.sender.send(Request {
            path: access_path.clone(),
            sender: tx,
        })?;
        rx.recv()?
    }

    fn multi_get(&self, access_paths: &[AccessPath]) -> Result<Vec<Option<Vec<u8>>>, Error> {
        access_paths
            .iter()
            .map(|path| StateView::get(self, path))
            .collect()
    }

    fn is_genesis(&self) -> bool {
        false
    }
}

pub fn access_path_into_ds(ap: AccessPath) -> DsAccessPath {
    DsAccessPath::new(ap.address.to_vec(), ap.path)
}

struct Request {
    path: AccessPath,
    sender: Sender<Result<Option<Vec<u8>>, Error>>,
}

impl RemoteCache for GrpcDataSource {
    fn get(&self, access_path: &AccessPath) -> VMResult<Option<Vec<u8>>> {
        StateView::get(self, access_path).map_err(|_| VMStatus::new(StatusCode::STORAGE_ERROR))
    }
}

impl Clear for GrpcDataSource {}

impl DataSource for GrpcDataSource {}
