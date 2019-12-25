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
        o: ::grpc::RequestOptions,
        p: VMExecuteRequest,
    ) -> ::grpc::SingleResponse<VMExecuteResponses> {
        return ::grpc::SingleResponse::err(::grpc::Error::Panic(String::from(
            "not implemented yet!",
        )));
    }

    fn get_imports(
        &self,
        o: ::grpc::RequestOptions,
        p: VMImportsRequest,
    ) -> ::grpc::SingleResponse<VMImportsResponses> {
        return ::grpc::SingleResponse::err(::grpc::Error::Panic(String::from(
            "not implemented yet!",
        )));
    }

    fn get_values(
        &self,
        o: ::grpc::RequestOptions,
        p: VMValuesRequest,
    ) -> ::grpc::SingleResponse<VMValuesResponses> {
        return ::grpc::SingleResponse::err(::grpc::Error::Panic(String::from(
            "not implemented yet!",
        )));
    }
}
