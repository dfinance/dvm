use std::sync::mpsc::{Sender, Receiver, channel};
use dvm_net::tonic::{transport::Server as TService, codegen::Pin};
use futures::Future;
use futures::task::{Context, Poll};
use std::sync::{Arc, Mutex};
use crate::{ArcMut, PORT_RANGE, Client};
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;
use tokio::runtime::Runtime;
use std::io::{ErrorKind, Error as IoError};
use std::mem;
use crate::compiled_protos::vm_grpc::vm_script_executor_server::VmScriptExecutorServer;
use crate::compiled_protos::vm_grpc::vm_module_publisher_server::VmModulePublisherServer;
use services::vm::VmService;
use data_source::MockDataSource;

pub struct Server {
    signal: Signal,
    port: u32,
    shutdown_monitor: Receiver<()>,
}

impl Server {
    pub fn new(data_source: MockDataSource) -> Server {
        let signal = Signal::new();
        let port = Arc::new(AtomicU32::new(0));
        let (shutdown_signal, shutdown_monitor) = channel();
        let service_port = port.clone();
        let service_signal = signal.clone();
        thread::spawn(move || {
            let mut rt = Runtime::new().unwrap();
            rt.block_on(async {
                for port in PORT_RANGE {
                    service_port.store(port, Ordering::SeqCst);
                    let service = VmService::new(data_source.clone(), None);
                    let service_res = TService::builder()
                        .add_service(VmScriptExecutorServer::new(service.clone()))
                        .add_service(VmModulePublisherServer::new(service.clone()))
                        .serve_with_shutdown(
                            format!("0.0.0.0:{}", port).parse().unwrap(),
                            service_signal.clone(),
                        )
                        .await;
                    match service_res {
                        Ok(_) => {
                            shutdown_signal.send(()).unwrap();
                            break;
                        }
                        Err(err) => {
                            if IoError::last_os_error().kind() == ErrorKind::AddrInUse {
                                continue;
                            } else {
                                eprintln!("err:{:?}", err);
                            }
                            shutdown_signal.send(()).unwrap();
                            break;
                        }
                    }
                }
            });
        });

        signal.ensure_run();

        Server {
            signal,
            port: port.load(Ordering::SeqCst),
            shutdown_monitor,
        }
    }

    pub fn port(&self) -> u32 {
        self.port
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.signal.shutdown();
        //We need to send something to the server so that the runtime calls the signal function and stops the process.
        // Otherwise, the service will continue to work in the background.
        if let Ok(client) = Client::new(self.port) {
            mem::forget(client);
        }
        self.shutdown_monitor.recv().unwrap();
    }
}

#[derive(Clone)]
pub struct Signal {
    shutdown_signal: Sender<()>,
    shutdown_signal_receiver: ArcMut<Receiver<()>>,
    start_signal: ArcMut<Option<Sender<()>>>,
    start_signal_receiver: ArcMut<Receiver<()>>,
}

impl Signal {
    pub fn new() -> Signal {
        let (shutdown_sender, shutdown_receiver) = channel();
        let (start_sender, start_receiver) = channel();

        Signal {
            shutdown_signal: shutdown_sender,
            shutdown_signal_receiver: Arc::new(Mutex::new(shutdown_receiver)),
            start_signal: Arc::new(Mutex::new(Some(start_sender))),
            start_signal_receiver: Arc::new(Mutex::new(start_receiver)),
        }
    }

    pub fn ensure_run(&self) {
        self.start_signal_receiver.lock().unwrap().recv().unwrap();
    }

    pub fn shutdown(&self) {
        self.shutdown_signal.send(()).unwrap();
    }
}

impl Future for Signal {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(start_signal) = self.start_signal.lock().unwrap().take() {
            if let Err(_err) = start_signal.send(()) {
                return Poll::Ready(());
            }
        }
        let receiver = self.shutdown_signal_receiver.lock().unwrap();
        match receiver.try_recv() {
            Ok(()) => Poll::Ready(()),
            Err(_) => Poll::Pending,
        }
    }
}

impl Default for Signal {
    fn default() -> Self {
        Self::new()
    }
}
