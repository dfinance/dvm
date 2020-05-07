use crate::StdError;
use crate::endpoint::*;
use crate::tonic;
use tonic::body::BoxBody;
use tower::Service;
use http::request::Request;
use http::response::Response;
use hyper::body::Body;

#[inline]
pub async fn serve_with<A, B>(
    router: tonic::transport::server::Router<A, B>,
    endpoint: Endpoint,
    should_close_on_drop: bool,
) -> Result<(), StdError>
where
    A: Service<Request<Body>, Response = Response<BoxBody>> + Clone + Send + 'static,
    A::Future: Send + 'static,
    A::Error: Into<StdError> + Send,
    B: Service<Request<Body>, Response = Response<BoxBody>> + Clone + Send + 'static,
    B::Future: Send + 'static,
    B::Error: Into<StdError> + Send,
{
    match endpoint {
        Endpoint::Http(http) => {
            router.serve(http.0).await?;
        }

        Endpoint::Ipc(ipc) => {
            use crate::transport::*;

            Ipc::create_dir_all(&ipc.0).await?;

            // TODO: are we should close on drop really?
            let mut uds = Listener::bind(&ipc.0)?.should_close_on_drop(should_close_on_drop);
            router.serve_with_incoming(uds.incoming()).await?;
        }
    }
    Ok(())
}

#[cfg(feature = "async-trait")]
pub use router_impl::*;

#[cfg(feature = "async-trait")]
mod router_impl {
    use super::*;

    extern crate async_trait;
    use async_trait::async_trait;
    use futures::TryFutureExt;

    #[async_trait]
    pub trait ServeWith {
        async fn serve_with(self, endpoint: Endpoint) -> Result<(), StdError>;
        async fn serve_with_anyway(self, endpoint: Endpoint) -> Result<(), StdError>;
    }
    #[async_trait]
    impl<A, B> ServeWith for tonic::transport::server::Router<A, B>
    where
        A: Service<Request<Body>, Response = Response<BoxBody>> + Clone + Send + 'static,
        A::Future: Send + 'static,
        A::Error: Into<StdError> + Send,
        B: Service<Request<Body>, Response = Response<BoxBody>> + Clone + Send + 'static,
        B::Future: Send + 'static,
        B::Error: Into<StdError> + Send,
    {
        #[inline]
        async fn serve_with(self, endpoint: Endpoint) -> Result<(), StdError> {
            serve_with(self, endpoint, false).await
        }

        #[inline]
        async fn serve_with_anyway(self, endpoint: Endpoint) -> Result<(), StdError> {
            use std::io::{Error as IoError, ErrorKind};
            let is_ipc = endpoint.is_ipc();
            let endpoint_clone = if is_ipc { Some(endpoint.clone()) } else { None };

            let result = serve_with(self, endpoint, false).await;

            match (is_ipc, result) {
                /* (true, Err(err)) if err.is::<IoError>() => {
                    match err.downcast_ref::<IoError>().map(|e| e.kind()) {
                        Some(ErrorKind::AddrInUse) | Some(ErrorKind::AlreadyExists) => {
                            use crate::transport::*;
                            let endpoint = endpoint_clone.unwrap();

                            let stream = Stream::connect(endpoint.to_string().parse().unwrap()).await.unwrap();
                            let mut channel = endpoint.connect().await.unwrap();
                            self.serve_with_incoming(stream......).await;
                            Err(err)
                        }
                        _ => Err(err),
                    }
                }, */
                (_, result) => result,
            }
        }
    }
}
