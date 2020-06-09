use std::task::{Context, Poll};

use futures_util::future;
use hyper::service::Service;
use crate::metrics::collector::MetricsCollector;
use std::time::Duration;
use std::net::SocketAddr;
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use crate::metrics::prometheus::encode_metrics;
use crate::heartbeat::HeartRateMonitor;

/// Instruction web service.
#[derive(Debug)]
pub struct InfoService {
    metric_collector: MetricsCollector,
    hrm: HeartRateMonitor,
}

impl InfoService {
    /// Returns the runtime metric in http format.
    fn load_metric(&mut self) -> Response<Body> {
        let metrics = self.metric_collector.get_metrics();
        let prometheus = encode_metrics(
            metrics,
            &[
                "ds_access",
                "compile",
                "multiple_compile",
                "script_metadata",
                "publish_module",
                "execute_script",
            ],
            true,
        );

        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/plain")
            .body(Body::from(prometheus))
            .unwrap()
    }

    /// Returns the health check status in http format.
    fn check_health(&mut self) -> Response<Body> {
        let status = if self.hrm.is_alive() { 200 } else { 500 };
        Response::builder()
            .status(status)
            .body(Body::empty())
            .unwrap()
    }
}

impl Service<Request<Body>> for InfoService {
    type Response = Response<Body>;
    type Error = hyper::Error;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        match (req.method(), req.uri().path()) {
            (&Method::GET, "/metrics") => future::ok(self.load_metric()),
            (&Method::GET, "/health") => future::ok(self.check_health()),
            _ => future::ok(
                Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::from(Vec::from(&b"Not found."[..])))
                    .unwrap(),
            ),
        }
    }
}

/// Info service maker.
pub struct ServiceMaker {
    metric_collector: MetricsCollector,
    hrm: HeartRateMonitor,
}

impl<T> Service<T> for ServiceMaker {
    type Response = InfoService;
    type Error = std::io::Error;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, _: T) -> Self::Future {
        future::ok(InfoService {
            metric_collector: self.metric_collector.clone(),
            hrm: self.hrm.clone(),
        })
    }
}

/// Starts a new information service.
pub async fn start_info_service(
    addr: SocketAddr,
    hrm: HeartRateMonitor,
    metrics_update_rate: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    let srv_maker = ServiceMaker {
        metric_collector: MetricsCollector::new(metrics_update_rate),
        hrm,
    };

    let server = Server::bind(&addr).serve(srv_maker);
    info!("Listening on http://{}", addr);
    server.await?;
    Ok(())
}
