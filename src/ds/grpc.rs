use core::cell::RefCell;
use std::borrow::BorrowMut;
use std::sync::{Arc, RwLock};
use std::sync::Mutex;
use std::thread;

use anyhow::Error;
use libra_state_view::StateView;
use libra_types::access_path::AccessPath;
use libra_types::account_address::AccountAddress;
use tokio::runtime::Runtime;
use tonic::transport::Channel;

use crate::compiled_protos::ds_grpc::ds_service_client::DsServiceClient;
use crate::compiled_protos::ds_grpc::DsAccessPath;

pub type ArcRuntime = Arc<Mutex<tokio::runtime::Runtime>>;

// TODO impl grpc data source
pub struct GrpcDataSource {
    runtime: ArcRuntime,
    client: Mutex<DsServiceClient<Channel>>,
    /// inner storage used for as temporary values
    inner: crate::ds::MockDataSource,
}

impl GrpcDataSource {
    pub fn new(runtime: ArcRuntime, uri: http::Uri) -> Self {
        let client = {
            let mut runtime = runtime.lock().unwrap();
            runtime
                .block_on(async { DsServiceClient::connect(uri).await })
                .expect("Cannot create DataSource client.")
        };

        Self {
            runtime,
            client: client.into(),
            inner: crate::ds::MockDataSource::default(),
        }
    }

    pub fn new_with(runtime: ArcRuntime, client: DsServiceClient<Channel>) -> Self {
        Self {
            runtime,
            client: client.into(),
            inner: crate::ds::MockDataSource::default(),
        }
    }

    pub fn get_blocking(&self, access_path: &AccessPath) -> Result<Option<Vec<u8>>, Error> {
        let request = tonic::Request::new(access_path.into());
            //        let mut mut_client = self.client.lock().unwrap().borrow_mut();

        //        let raw_request_future = mut_client.get_raw(request);

        println!("trying to fetch module for access path");
        //        let mut locked_runtime = self.runtime.lock().unwrap();
        println!("after locking runtime");

        let res = crossbeam::scope(move |s| {
            s.spawn(move |_| {
                let mut mut_client = self.client.lock().unwrap();
                let mut locked_runtime = self.runtime.lock().unwrap();
                locked_runtime.block_on(mut_client.get_raw(request))
            }).join().unwrap()
        })
        .unwrap();
//        let res = thread::spawn(move || {
//                let mut mut_client = self.client.lock().unwrap();
//                let mut locked_runtime = self.runtime.lock().unwrap();
//                locked_runtime.block_on(mut_client.get_raw(request))
//        }).join().unwrap();
        dbg!(&res);
        //        let res = thread::spawn(|| {
        //            let mut mut_client = self.client.lock().unwrap();
        //            let mut locked_runtime = self.runtime.lock().unwrap();
        //            locked_runtime.block_on(mut_client.get_raw(request))
        //        })
        //        .join()
        //        .unwrap();
        //        let res = locked_runtime.block_on(raw_request_future);
        dbg!("after block_on in get_raw");

        Ok(res
            .map_err(|err| {
                // TODO: normally log error and/or panic
                println!("DataSource client unexpected error: {:?}", err);
                err
            })
            .map(|res| res.into_inner().blob)
            .ok())
    }

    // TODO: XXX: impl this
    // pub fn multi_get_blocking(
    //     &self,
    //     access_paths: &[AccessPath],
    // ) -> Result<Option<Vec<Vec<u8>>>, Error> {
    //     let request = tonic::Request::new(DsAccessPaths {
    //         paths: access_paths.into_iter().map(|ap| ap.into()).collect(),
    //     });
    //     let res = self
    //         .runtime
    //         .lock()
    //         .unwrap()
    //         .block_on(self.client.borrow_mut().multi_get_raw(request));

    //     Ok(res
    //         .map_err(|err| {
    //             // TODO: normally log error and/or panic
    //             println!("DataSource client unexpected error: {:?}", err);
    //             err
    //         })
    //         .map(|res| res.into_inner().blobs)
    //         .ok())
    // }

    pub fn get_blocking_test(&self) {
        let ap = AccessPath::new(AccountAddress::new([0_u8; 32]), Vec::new());
        let result = self.get_blocking(&ap);
        println!("RESULT: {:?}", result);
    }
}

impl StateView for GrpcDataSource {
    fn get(&self, access_path: &AccessPath) -> Result<Option<Vec<u8>>, Error> {
        dbg!("self.get_blocking(access_path)");
        self.get_blocking(access_path)
        //        thread::spawn(|| &self.get_blocking(access_path))
        //            .join()
        //            .unwrap()
        //        self.get_blocking(access_path)
    }

    fn multi_get(&self, _access_paths: &[AccessPath]) -> Result<Vec<Option<Vec<u8>>>, Error> {
        // TODO: self.multi_get_blocking(access_paths)
        unimplemented!();
    }

    fn is_genesis(&self) -> bool {
        self.inner.is_genesis()
    }
}

// impl From<libra_types::access_path::AccessPath> for crate::grpc::ds_service_client::AccessPath {
impl From<AccessPath> for DsAccessPath {
    fn from(path: AccessPath) -> Self {
        Self {
            address: path.address.to_vec(),
            path: path.path,
        }
    }
}

impl<'a> From<&'a AccessPath> for DsAccessPath {
    fn from(path: &'a AccessPath) -> Self {
        Self {
            address: path.address.to_vec(),
            path: path.path.to_vec(),
        }
    }
}
