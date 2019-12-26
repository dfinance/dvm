use crate::grpc::vm::*;
use crate::grpc::vm_grpc::VMService;

pub struct VM {
    // TODO
// state:
}

impl VM {
    pub fn new() -> Self {
        Self {}
    }
}

impl VMService for VM {
    fn execute_contracts(
        &self,
        _o: ::grpc::RequestOptions,
        _p: VMExecuteRequest,
    ) -> ::grpc::SingleResponse<VMExecuteResponses> {
        ::grpc::SingleResponse::err(::grpc::Error::Panic(String::from("not implemented yet!")))
    }

    fn get_imports(
        &self,
        _o: ::grpc::RequestOptions,
        _p: VMImportsRequest,
    ) -> ::grpc::SingleResponse<VMImportsResponses> {
        ::grpc::SingleResponse::err(::grpc::Error::Panic(String::from("not implemented yet!")))
    }

    fn get_values(
        &self,
        _o: ::grpc::RequestOptions,
        _p: VMValuesRequest,
    ) -> ::grpc::SingleResponse<VMValuesResponses> {
        ::grpc::SingleResponse::err(::grpc::Error::Panic(String::from("not implemented yet!")))
    }
}
