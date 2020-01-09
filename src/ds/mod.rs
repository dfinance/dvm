mod grpc;
mod mock;

use anyhow::Error;
use libra_types::write_set::WriteSet;

pub use mock::MockDataSource;

pub trait MergeWriteSet {
    fn merge_write_set(&mut self, write_set: WriteSet) -> Result<(), Error>;
}
