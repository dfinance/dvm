/// Status of code contract execution.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct VmErrorStatus {
    /// Major error status.
    #[prost(uint64, tag = "1")]
    pub major_status: u64,
    /// Sub status if needed (optional).
    #[prost(uint64, tag = "2")]
    pub sub_status: u64,
    /// Message with error details if needed (optional).
    #[prost(string, tag = "3")]
    pub message: std::string::String,
}
/// Describing VMType for events.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct VmType {
    /// Type.
    #[prost(enumeration = "VmTypeTag", tag = "1")]
    pub tag: i32,
    /// If type is Struct put struct into variable, otherwise not, optional value.
    #[prost(message, optional, tag = "2")]
    pub struct_tag: ::std::option::Option<VmStructTag>,
}
/// Structure tag (for vm events contains structures).
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct VmStructTag {
    /// address of module owner
    #[prost(bytes, tag = "1")]
    pub address: std::vec::Vec<u8>,
    /// module where event happens.
    #[prost(string, tag = "2")]
    pub module: std::string::String,
    /// name of event (not sure yet, need to test).
    #[prost(string, tag = "3")]
    pub name: std::string::String,
    /// event parameters (recursive).
    #[prost(message, repeated, tag = "4")]
    pub type_params: ::std::vec::Vec<VmType>,
}
/// VM event returns after contract execution.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct VmEvent {
    /// key to store vm event.
    #[prost(bytes, tag = "1")]
    pub key: std::vec::Vec<u8>,
    /// sequence number of event during execution.
    #[prost(uint64, tag = "2")]
    pub sequence_number: u64,
    /// Type of value inside event.
    #[prost(message, optional, tag = "3")]
    pub r#type: ::std::option::Option<VmType>,
    /// Event data in bytes to parse.
    #[prost(bytes, tag = "4")]
    pub event_data: std::vec::Vec<u8>,
}
/// Storage path
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct VmAccessPath {
    /// account address.
    #[prost(bytes, tag = "1")]
    pub address: std::vec::Vec<u8>,
    /// storage path.
    #[prost(bytes, tag = "2")]
    pub path: std::vec::Vec<u8>,
}
/// VM value should be passed before execution and return after execution (with opcodes), write_set in nutshell.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct VmValue {
    /// Type of operation
    #[prost(enumeration = "VmWriteOp", tag = "2")]
    pub r#type: i32,
    /// Value returns from vm.
    #[prost(bytes, tag = "1")]
    pub value: std::vec::Vec<u8>,
    /// Access path.
    #[prost(message, optional, tag = "3")]
    pub path: ::std::option::Option<VmAccessPath>,
}
/// Contract arguments.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct VmArgs {
    /// Argument type.
    #[prost(enumeration = "VmTypeTag", tag = "1")]
    pub r#type: i32,
    /// Argument value.
    #[prost(string, tag = "2")]
    pub value: std::string::String,
}
/// VM contract object to process.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct VmContract {
    /// owner of contract (module) or script executor.
    #[prost(bytes, tag = "1")]
    pub address: std::vec::Vec<u8>,
    /// maximal total gas specified by wallet to spend for this transaction.
    #[prost(uint64, tag = "2")]
    pub max_gas_amount: u64,
    /// maximal price can be paid per gas.
    #[prost(uint64, tag = "3")]
    pub gas_unit_price: u64,
    /// compiled contract code.
    #[prost(bytes, tag = "4")]
    pub code: std::vec::Vec<u8>,
    /// Type of contract
    #[prost(enumeration = "ContractType", tag = "6")]
    pub contract_type: i32,
    /// Contract arguments.
    #[prost(message, repeated, tag = "7")]
    pub args: ::std::vec::Vec<VmArgs>,
}
/// Response from VM contains write_set, events, gas used and status for specific contract.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct VmExecuteResponse {
    /// using string instead of bytes for now, as map support only ints and strings as keys
    #[prost(message, repeated, tag = "1")]
    pub write_set: ::std::vec::Vec<VmValue>,
    /// list of events executed during contract execution
    #[prost(message, repeated, tag = "2")]
    pub events: ::std::vec::Vec<VmEvent>,
    /// Gas used during execution.
    #[prost(uint64, tag = "3")]
    pub gas_used: u64,
    /// Status of contract execution.
    #[prost(enumeration = "ContractStatus", tag = "4")]
    pub status: i32,
    /// Status in case of error.
    #[prost(message, optional, tag = "5")]
    pub status_struct: ::std::option::Option<VmErrorStatus>,
}
/// Response from VM in case of execution multiplay contracts.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct VmExecuteResponses {
    /// Result of executions.
    #[prost(message, repeated, tag = "1")]
    pub executions: ::std::vec::Vec<VmExecuteResponse>,
}
/// Execute request for VM
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct VmExecuteRequest {
    /// contracts to execute.
    #[prost(message, repeated, tag = "1")]
    pub contracts: ::std::vec::Vec<VmContract>,
    /// options to execute.
    #[prost(uint64, tag = "4")]
    pub options: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MvIrSourceFile {
    #[prost(string, tag = "1")]
    pub text: std::string::String,
    #[prost(bytes, tag = "2")]
    pub address: std::vec::Vec<u8>,
    #[prost(enumeration = "ContractType", tag = "3")]
    pub r#type: i32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CompilationResult {
    #[prost(bytes, tag = "1")]
    pub bytecode: std::vec::Vec<u8>,
    #[prost(string, repeated, tag = "2")]
    pub errors: ::std::vec::Vec<std::string::String>,
}
/// Type of contract (module or script).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum ContractType {
    /// If VM works with module.
    Module = 0,
    /// If VM works with script.
    Script = 1,
}
/// Status of contract execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum ContractStatus {
    /// If transaction should be ignored, because of error.
    Discard = 0,
    /// If we keep transaction and write write_set.
    Keep = 1,
}
/// Type of value returned by event during contract execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum VmTypeTag {
    /// Bool
    Bool = 0,
    /// Uint64
    U64 = 1,
    /// Bytes
    ByteArray = 2,
    /// Address
    Address = 3,
    /// Structure (could be several arguments for event call).
    Struct = 4,
    /// U8
    U8 = 5,
    /// U128
    U128 = 6,
}
/// Write set operation type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum VmWriteOp {
    /// Insert or update value
    Value = 0,
    /// Delete.
    Deletion = 1,
}
#[doc = r" Generated client implementations."]
pub mod vm_service_client {
    #![allow(unused_variables, dead_code, missing_docs)]
    use tonic::codegen::*;
    #[doc = " GRPC service"]
    pub struct VmServiceClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl VmServiceClient<tonic::transport::Channel> {
        #[doc = r" Attempt to create a new client by connecting to a given endpoint."]
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: std::convert::TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> VmServiceClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::ResponseBody: Body + HttpBody + Send + 'static,
        T::Error: Into<StdError>,
        <T::ResponseBody as HttpBody>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_interceptor(inner: T, interceptor: impl Into<tonic::Interceptor>) -> Self {
            let inner = tonic::client::Grpc::with_interceptor(inner, interceptor);
            Self { inner }
        }
        pub async fn execute_contracts(
            &mut self,
            request: impl tonic::IntoRequest<super::VmExecuteRequest>,
        ) -> Result<tonic::Response<super::VmExecuteResponses>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/vm_grpc.VMService/ExecuteContracts");
            self.inner.unary(request.into_request(), path, codec).await
        }
    }
    impl<T: Clone> Clone for VmServiceClient<T> {
        fn clone(&self) -> Self {
            Self {
                inner: self.inner.clone(),
            }
        }
    }
}
#[doc = r" Generated client implementations."]
pub mod vm_compiler_client {
    #![allow(unused_variables, dead_code, missing_docs)]
    use tonic::codegen::*;
    pub struct VmCompilerClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl VmCompilerClient<tonic::transport::Channel> {
        #[doc = r" Attempt to create a new client by connecting to a given endpoint."]
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: std::convert::TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> VmCompilerClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::ResponseBody: Body + HttpBody + Send + 'static,
        T::Error: Into<StdError>,
        <T::ResponseBody as HttpBody>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_interceptor(inner: T, interceptor: impl Into<tonic::Interceptor>) -> Self {
            let inner = tonic::client::Grpc::with_interceptor(inner, interceptor);
            Self { inner }
        }
        pub async fn compile(
            &mut self,
            request: impl tonic::IntoRequest<super::MvIrSourceFile>,
        ) -> Result<tonic::Response<super::CompilationResult>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/vm_grpc.VMCompiler/Compile");
            self.inner.unary(request.into_request(), path, codec).await
        }
    }
    impl<T: Clone> Clone for VmCompilerClient<T> {
        fn clone(&self) -> Self {
            Self {
                inner: self.inner.clone(),
            }
        }
    }
}
#[doc = r" Generated server implementations."]
pub mod vm_service_server {
    #![allow(unused_variables, dead_code, missing_docs)]
    use tonic::codegen::*;
    #[doc = "Generated trait containing gRPC methods that should be implemented for use with VmServiceServer."]
    #[async_trait]
    pub trait VmService: Send + Sync + 'static {
        async fn execute_contracts(
            &self,
            request: tonic::Request<super::VmExecuteRequest>,
        ) -> Result<tonic::Response<super::VmExecuteResponses>, tonic::Status>;
    }
    #[doc = " GRPC service"]
    #[derive(Debug)]
    #[doc(hidden)]
    pub struct VmServiceServer<T: VmService> {
        inner: _Inner<T>,
    }
    struct _Inner<T>(Arc<T>, Option<tonic::Interceptor>);
    impl<T: VmService> VmServiceServer<T> {
        pub fn new(inner: T) -> Self {
            let inner = Arc::new(inner);
            let inner = _Inner(inner, None);
            Self { inner }
        }
        pub fn with_interceptor(inner: T, interceptor: impl Into<tonic::Interceptor>) -> Self {
            let inner = Arc::new(inner);
            let inner = _Inner(inner, Some(interceptor.into()));
            Self { inner }
        }
    }
    impl<T: VmService> Service<http::Request<HyperBody>> for VmServiceServer<T> {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = Never;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<HyperBody>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/vm_grpc.VMService/ExecuteContracts" => {
                    struct ExecuteContractsSvc<T: VmService>(pub Arc<T>);
                    impl<T: VmService> tonic::server::UnaryService<super::VmExecuteRequest> for ExecuteContractsSvc<T> {
                        type Response = super::VmExecuteResponses;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::VmExecuteRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { inner.execute_contracts(request).await };
                            Box::pin(fut)
                        }
                    }
                    let inner = self.inner.clone();
                    let fut = async move {
                        let interceptor = inner.1.clone();
                        let inner = inner.0;
                        let method = ExecuteContractsSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = if let Some(interceptor) = interceptor {
                            tonic::server::Grpc::with_interceptor(codec, interceptor)
                        } else {
                            tonic::server::Grpc::new(codec)
                        };
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => Box::pin(async move {
                    Ok(http::Response::builder()
                        .status(200)
                        .header("grpc-status", "12")
                        .body(tonic::body::BoxBody::empty())
                        .unwrap())
                }),
            }
        }
    }
    impl<T: VmService> Clone for VmServiceServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self { inner }
        }
    }
    impl<T: VmService> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(self.0.clone(), self.1.clone())
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: VmService> tonic::transport::NamedService for VmServiceServer<T> {
        const NAME: &'static str = "vm_grpc.VMService";
    }
}
#[doc = r" Generated server implementations."]
pub mod vm_compiler_server {
    #![allow(unused_variables, dead_code, missing_docs)]
    use tonic::codegen::*;
    #[doc = "Generated trait containing gRPC methods that should be implemented for use with VmCompilerServer."]
    #[async_trait]
    pub trait VmCompiler: Send + Sync + 'static {
        async fn compile(
            &self,
            request: tonic::Request<super::MvIrSourceFile>,
        ) -> Result<tonic::Response<super::CompilationResult>, tonic::Status>;
    }
    #[derive(Debug)]
    #[doc(hidden)]
    pub struct VmCompilerServer<T: VmCompiler> {
        inner: _Inner<T>,
    }
    struct _Inner<T>(Arc<T>, Option<tonic::Interceptor>);
    impl<T: VmCompiler> VmCompilerServer<T> {
        pub fn new(inner: T) -> Self {
            let inner = Arc::new(inner);
            let inner = _Inner(inner, None);
            Self { inner }
        }
        pub fn with_interceptor(inner: T, interceptor: impl Into<tonic::Interceptor>) -> Self {
            let inner = Arc::new(inner);
            let inner = _Inner(inner, Some(interceptor.into()));
            Self { inner }
        }
    }
    impl<T: VmCompiler> Service<http::Request<HyperBody>> for VmCompilerServer<T> {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = Never;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<HyperBody>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/vm_grpc.VMCompiler/Compile" => {
                    struct CompileSvc<T: VmCompiler>(pub Arc<T>);
                    impl<T: VmCompiler> tonic::server::UnaryService<super::MvIrSourceFile> for CompileSvc<T> {
                        type Response = super::CompilationResult;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::MvIrSourceFile>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { inner.compile(request).await };
                            Box::pin(fut)
                        }
                    }
                    let inner = self.inner.clone();
                    let fut = async move {
                        let interceptor = inner.1.clone();
                        let inner = inner.0;
                        let method = CompileSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = if let Some(interceptor) = interceptor {
                            tonic::server::Grpc::with_interceptor(codec, interceptor)
                        } else {
                            tonic::server::Grpc::new(codec)
                        };
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => Box::pin(async move {
                    Ok(http::Response::builder()
                        .status(200)
                        .header("grpc-status", "12")
                        .body(tonic::body::BoxBody::empty())
                        .unwrap())
                }),
            }
        }
    }
    impl<T: VmCompiler> Clone for VmCompilerServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self { inner }
        }
    }
    impl<T: VmCompiler> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(self.0.clone(), self.1.clone())
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: VmCompiler> tonic::transport::NamedService for VmCompilerServer<T> {
        const NAME: &'static str = "vm_grpc.VMCompiler";
    }
}
