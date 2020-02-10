use std::sync::mpsc;
use libra_state_view::StateView;
use libra_types::access_path::AccessPath;
use crate::grpc::*;
use anyhow::Error;

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

pub struct CachingDataSource<K: Send, V: Send> {
    remote: ChannelDataSource<K, V>,
    /* // TODO: inpl caching
    /// inner storage used for as temporary values
    storage: HashMap<AccessPath, Vec<u8>>, */
}

impl CachingDataSource<Request, Response> {
    pub fn new(tx: mpsc::Sender<Request>, rx: mpsc::Receiver<Response>) -> Self {
        Self {
            remote: ChannelDataSource::new(tx, rx),
            // storage: Default::default(),
        }
    }
}

impl StateView for CachingDataSource<Request, Response> {
    fn get(&self, access_path: &Request) -> Result<Option<Vec<u8>>, Error> {
        let response = self.remote.get_blocking(access_path)?;
        match response.error_code {
            // if no error code, return blob
            0 => Ok(Some(response.blob)),
            // if BadRequest, return Err()
            1 => Err(anyhow!(String::from_utf8(response.error_message).unwrap())),
            // if NoData, return None
            2 => Ok(None),
            _ => panic!("No such value for ErrorCode enum"),
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
