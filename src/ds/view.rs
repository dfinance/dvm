use std::sync::{mpsc, Arc, Mutex};
use libra::{libra_types, libra_state_view};
use libra_state_view::StateView;
use libra_types::access_path::AccessPath;
use anyhow::Error;
use crate::compiled_protos::ds_grpc::DsRawResponse;
use crate::compiled_protos::ds_grpc::ds_raw_response::ErrorCode;

pub type Request = AccessPath;
pub type Response = DsRawResponse;

pub struct ChannelDataSource<K: Send, V: Send> {
    tx: mpsc::Sender<K>,
    rx: mpsc::Receiver<V>,
}

impl<K: Send, V: Send> ChannelDataSource<K, V> {
    pub fn new(tx: mpsc::Sender<K>, rx: mpsc::Receiver<V>) -> Self {
        Self { tx, rx }
    }
}

impl<K: 'static + Send + Sync + Clone, V: Send> ChannelDataSource<K, V> {
    pub fn get_blocking(&self, key: &K) -> Result<V, Error> {
        // self.tx.send(key.to_owned())?;
        // self.rx.recv().map_err(|err| err.into())
        self.tx
            .send(key.clone())
            .map_err(Error::from)
            .and_then(|_| self.rx.recv().map_err(Error::from))
    }
}

#[derive(Clone)]
pub struct CachingDataSource<K: Send, V: Send> {
    remote: Arc<Mutex<ChannelDataSource<K, V>>>,
    /* // TODO: inpl caching
    /// inner storage used for as temporary values
    storage: HashMap<AccessPath, Vec<u8>>, */
}

impl CachingDataSource<Request, Response> {
    pub fn new(tx: mpsc::Sender<Request>, rx: mpsc::Receiver<Response>) -> Self {
        Self {
            remote: Arc::new(Mutex::new(ChannelDataSource::new(tx, rx))),
            // storage: Default::default(),
        }
    }
}

impl StateView for CachingDataSource<Request, Response> {
    fn get(&self, access_path: &Request) -> Result<Option<Vec<u8>>, Error> {
        let response = self.remote.lock().unwrap().get_blocking(access_path)?;
        let error_code =
            ErrorCode::from_i32(response.error_code).expect("Invalid ErrorCode enum value");
        match error_code {
            // if no error code, return blob
            ErrorCode::None => Ok(Some(response.blob)),
            // if BadRequest, return Err()
            ErrorCode::BadRequest => Err(anyhow!(response.error_message)),
            // if NoData, return None
            ErrorCode::NoData => Ok(None),
        }
    }

    fn multi_get(&self, _access_paths: &[AccessPath]) -> Result<Vec<Option<Vec<u8>>>, Error> {
        // TODO: self.multi_get_blocking(access_paths)
        unimplemented!();
    }

    fn is_genesis(&self) -> bool {
        // self.inner.is_genesis()
        unimplemented!();
    }
}
