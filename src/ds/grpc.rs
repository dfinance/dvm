use crate::ds::MergeWriteSet;
use libra_state_view::StateView;
use libra_types::access_path::AccessPath;
use anyhow::Error;
use libra_types::write_set::WriteSet;

// TODO impl grpc data source
pub struct GrpcDataSource {}

impl StateView for GrpcDataSource {
    fn get(&self, _access_path: &AccessPath) -> Result<Option<Vec<u8>>, Error> {
        unimplemented!()
    }

    fn multi_get(&self, _access_paths: &[AccessPath]) -> Result<Vec<Option<Vec<u8>>>, Error> {
        unimplemented!()
    }

    fn is_genesis(&self) -> bool {
        false
    }
}

impl MergeWriteSet for GrpcDataSource {
    fn merge_write_set(&mut self, _write_set: WriteSet) -> Result<(), Error> {
        unimplemented!()
    }
}
