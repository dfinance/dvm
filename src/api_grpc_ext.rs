use libra::libra_types;
use libra_types::access_path::AccessPath;
use dvm_api::grpc::ds_grpc::DsAccessPath;

pub fn access_path_into_ds(ap: AccessPath) -> DsAccessPath {
    DsAccessPath::new(ap.address.to_vec(), ap.path)
}
