use crate::compiled_protos::ds_grpc::DsAccessPath;
use libra_types::access_path::AccessPath;

pub mod ds_grpc;
pub mod vm_grpc;

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
