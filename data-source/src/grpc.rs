use libra::{libra_types, libra_state_view};
use libra_state_view::StateView;
use libra_types::access_path::AccessPath;
use anyhow::Error;
use http::Uri;
use std::thread::JoinHandle;
use std::sync::Arc;
use tokio::runtime::Runtime;
use std::thread;
use crossbeam::channel::{Sender, Receiver, bounded};
use std::time::Duration;
use dvm_api::grpc::ds_grpc::{
    ds_service_client::DsServiceClient, DsAccessPath, ds_raw_response::ErrorCode,
};

use dvm_api::tonic;
use std::process::exit;

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
            loop {
                match DsServiceClient::connect(ds_addr.clone()).await {
                    Ok(client) => return client,
                    Err(_) => tokio::time::delay_for(Duration::from_secs(1)).await,
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
                        error!("Failed to send request to data source: {:?}", err);
                        exit(-1);
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
        access_paths.iter().map(|path| self.get(path)).collect()
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
