use std::sync::mpsc;
use libra_state_view::StateView;
use libra_types::access_path::AccessPath;
use anyhow::Error;

use crate::ds::mock::MockDataSource;

pub type Request = AccessPath;
pub type Response = Option<Vec<u8>>;

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
    /// inner storage used for as temporary values
    /* TODO: inpl caching */
    #[allow(dead_code)]
    storage: MockDataSource,
}

impl CachingDataSource<Request, Response> {
    pub fn new(tx: mpsc::Sender<Request>, rx: mpsc::Receiver<Response>) -> Self {
        Self {
            remote: ChannelDataSource::new(tx, rx),
            storage: MockDataSource::default(),
        }
    }
}

impl StateView for CachingDataSource<Request, Response> {
    fn get(&self, access_path: &Request) -> Result<Response, Error> {
        self.remote.get_blocking(access_path)
    }

    fn multi_get(&self, _access_paths: &[AccessPath]) -> Result<Vec<Response>, Error> {
        // TODO: self.multi_get_blocking(access_paths)
        unimplemented!();
    }

    fn is_genesis(&self) -> bool {
        // self.inner.is_genesis()
        unimplemented!();
    }
}
