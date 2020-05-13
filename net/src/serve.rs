use crate::StdError;
use crate::endpoint::*;
use crate::transport::Guard;
use crate::tonic;
use tonic::body::BoxBody;
use tower::Service;
use futures::Future;
use http::request::Request;
use http::response::Response;
use hyper::body::Body;

pub async fn serve_with_drop<A, B>(
    router: tonic::transport::server::Router<A, B>,
    endpoint: Endpoint,
    should_close_on_drop: bool,
) -> Result<Option<Guard>, StdError>
where
    A: Service<Request<Body>, Response = Response<BoxBody>> + Clone + Send + 'static,
    A::Future: Send + 'static,
    A::Error: Into<StdError> + Send,
    B: Service<Request<Body>, Response = Response<BoxBody>> + Clone + Send + 'static,
    B::Future: Send + 'static,
    B::Error: Into<StdError> + Send,
{
    Ok(match endpoint {
        Endpoint::Http(http) => router.serve(http.0).await.map(|_| None)?,

        Endpoint::Ipc(ipc) => {
            use crate::transport::*;

            Ipc::create_dir_all(&ipc.0).await?;

            // TODO: are we should close on drop really?
            let mut uds = Listener::bind(&ipc.0)?.guarded(should_close_on_drop);
            let guard = uds.guard();
            router
                .serve_with_incoming(uds.incoming())
                .await
                .map(move |_| guard)?
        }
    })
}

pub async fn serve_with_shutdown<A, B, F>(
    router: tonic::transport::server::Router<A, B>,
    endpoint: Endpoint,
    signal: F,
) -> Result<Option<Guard>, StdError>
where
    A: Service<Request<Body>, Response = Response<BoxBody>> + Clone + Send + 'static,
    A::Future: Send + 'static,
    A::Error: Into<StdError> + Send,
    B: Service<Request<Body>, Response = Response<BoxBody>> + Clone + Send + 'static,
    B::Future: Send + 'static,
    B::Error: Into<StdError> + Send,
    F: Future<Output = ()>,
{
    Ok(match endpoint {
        Endpoint::Http(http) => router.serve(http.0).await.map(|_| None)?,

        Endpoint::Ipc(ipc) => {
            use crate::transport::*;

            Ipc::create_dir_all(&ipc.0).await?;

            // TODO: are we should close on drop really?
            let mut uds = Listener::bind(&ipc.0)?.guarded(true);
            let guard = uds.guard();
            router
                .serve_with_incoming_shutdown(uds.incoming(), signal)
                .await
                .map(move |_| guard)?
        }
    })
}

pub use router_impl::*;

mod router_impl {
    use super::*;

    #[tonic::async_trait]
    pub trait ServeWith {
        async fn serve_ext(self, endpoint: Endpoint) -> Result<Option<Guard>, StdError>;
        async fn serve_ext_with_shutdown<F: Future<Output = ()> + Send>(
            self,
            endpoint: Endpoint,
            signal: F,
        ) -> Result<Option<Guard>, StdError>;
    }

    #[tonic::async_trait]
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
        async fn serve_ext(self, endpoint: Endpoint) -> Result<Option<Guard>, StdError> {
            serve_with_drop(self, endpoint, true).await
        }

        #[inline]
        async fn serve_ext_with_shutdown<F>(
            self,
            endpoint: Endpoint,
            signal: F,
        ) -> Result<Option<Guard>, StdError>
        where
            F: Future<Output = ()> + Send,
        {
            serve_with_shutdown(self, endpoint, signal).await
        }
    }
}
