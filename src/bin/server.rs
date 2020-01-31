//! Server implementation on tonic & tokio.
//! Run with `cargo run --bin server "[::1]:50051" "http://[::1]:50052"`
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;
use std::thread;
use std::thread::JoinHandle;

use crossbeam;
use http::Uri;
use structopt::StructOpt;
use tokio::runtime;
use tokio::runtime::Runtime;
use tonic::transport::Server;

use move_vm_in_cosmos::compiled_protos::vm_grpc::vm_service_server::VmServiceServer;
use move_vm_in_cosmos::ds::GrpcDataSource;
use move_vm_in_cosmos::ds::MockDataSource;
use move_vm_in_cosmos::grpc;
use move_vm_in_cosmos::service::MoveVmService;
use move_vm_in_cosmos::test_kit::ArcMut;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::time::Duration;

#[derive(Debug, StructOpt, Clone)]
struct Options {
    #[structopt(help = "Address in the form of HOST_ADDRESS:PORT")]
    address: SocketAddr,

    #[structopt(help = "DataSource Server internet address")]
    ds: Uri,
}

#[derive(Clone)]
pub struct Signal {
    shutdown_signal: Sender<()>,
    shutdown_signal_receiver: ArcMut<Receiver<()>>,
}

impl Signal {
    pub fn new() -> Signal {
        let (shutdown_sender, shutdown_receiver) = channel();

        Signal {
            shutdown_signal: shutdown_sender,
            shutdown_signal_receiver: Arc::new(Mutex::new(shutdown_receiver)),
        }
    }

//    pub fn ensure_run(&self) {
//        self.start_signal_receiver.lock().unwrap().recv().unwrap();
//    }

    pub fn shutdown(&self) {
        self.shutdown_signal.send(()).unwrap();
    }
}

impl Future for Signal {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
//        if let Some(start_signal) = self.start_signal.lock().unwrap().take() {
//            if let Err(_err) = start_signal.send(()) {
//                return Poll::Ready(());
//            }
//        }
        let receiver = self.shutdown_signal_receiver.lock().unwrap();
        match receiver.try_recv() {
            Ok(()) => Poll::Ready(()),
            Err(_) => Poll::Pending,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = runtime::Builder::new()
        .core_threads(4)
        .threaded_scheduler()
        .enable_all()
        .build()?;
    let runtime = Arc::new(Mutex::new(runtime));

    let options = Options::from_args();

    let ws = MockDataSource::default();

    let ds = {
        let connected_ds_client = {
            println!("Connecting to data-source: {}", options.ds);

            let ds_uri = options.ds;
            use move_vm_in_cosmos::compiled_protos::ds_grpc::ds_service_client::DsServiceClient;
            let connected_client = {
                let mut locked_runtime = runtime.lock().unwrap();
                locked_runtime.block_on(async { DsServiceClient::connect(ds_uri).await })?
            };
            println!("client got connected to ds-server");
            connected_client
        };
        GrpcDataSource::new_with(Arc::clone(&runtime), connected_ds_client)
    };
    let service = MoveVmService::with_auto_commit(Box::new(ds), Box::new(ws));

    println!("Listening on {}", options.address);
    let bind_addr = options.address;
    //    let (task, handle) = task::joinable(BlockingTask::new(async move {
    //        Server::builder()
    //            .add_service(VmServiceServer::new(service))
    //            .serve(bind_addr)
    //            .await
    //    }));

    //    let handle = runtime.lock().unwrap().spawn_blocking(async move {
    //        Server::builder()
    //            .add_service(VmServiceServer::new(service))
    //            .serve(bind_addr)
    //            .await
    //    }).await;
    //    let handle = runtime.lock().unwrap().block_on(async move {
    //        Server::builder()
    //            .add_service(VmServiceServer::new(service))
    //            .serve(bind_addr)
    //            .await
    //    });
    //    handle.unwrap();
    //
//    let signal = Signal::new();
//    let (tx, rx) = channel();
//    let cloned_signal = signal.clone();
    thread::spawn(move || {
        runtime.lock().unwrap().spawn(async move {
            Server::builder()
                .add_service(VmServiceServer::new(service))
                .serve(bind_addr)
                .await
        });
    });

//    ctrlc::set_handler(move || {
//        signal.shutdown();
//    });

    let th = thread::spawn(move || {
        thread::sleep(Duration::new(100, 0));
//        rx.recv().unwrap();
    });
    th.join().unwrap();

    //
    //    tokio::spawn(async move {
    //        tx.send(poll_fn(blocking(|| code())).await).unwrap();
    //    });
    //    rx.await.unwrap()

    //    crossbeam::scope(move |s| {
    //        s.spawn(move |_| {
    //            let mut rt = runtime::Builder::new()
    //                .thread_name("grpc server reactor")
    //                .basic_scheduler()
    //                .enable_all()
    //                .build()
    //                .unwrap();
    //            rt.block_on(async move {
    //                Server::builder()
    //                    .add_service(VmServiceServer::new(service))
    //                    .serve(bind_addr)
    //                    .await
    //            })
    //            .unwrap();
    //        });
    //    }).unwrap();
    //    let th = thread::spawn(move || {
    //        let mut rt = runtime::Builder::new()
    //            .thread_name("grpc server reactor")
    //            .basic_scheduler()
    //            .build()
    //            .unwrap();
    //        rt.block_on(async move {
    //            Server::builder()
    //                .add_service(VmServiceServer::new(service))
    //                .serve(bind_addr)
    //                .await
    //        })
    //        .unwrap();
    //    });
    //    th.join().unwrap();
    //    runtime.lock().unwrap().block_on(async move {
    //        Server::builder()
    //            .add_service(VmServiceServer::new(service))
    //            .serve(bind_addr)
    //            .await
    //    }).unwrap();

    Ok(())
}
