extern crate futures;
extern crate futures_cpupool;
extern crate grpc;
extern crate protobuf;
extern crate tls_api;

use std::env;
use std::net::ToSocketAddrs;
use std::thread;

use crate::grpc::vm::*;
use crate::grpc::vm_grpc::*;

// TODO: impl normal Error instead of use String
type Error = String;

/// Run the server.
/// Method not blocking current thread.
pub fn run_async<T>(service: T) -> Result<grpc::Server, Error>
where
    T: 'static + std::marker::Sync + std::marker::Send + VMService,
{
    let cfg = get_cfg_vars()?;

    let mut server = ::grpc::ServerBuilder::new_plain();

    server
        .http
        .set_addr(cfg.address)
        .map_err(|err| format!("{:?}", err))?;

    server.add_service(VMServiceServer::new_service_def(service));

    let server = server.build().map_err(|err| format!("{:?}", err))?;
    println!("Server '{}' launched, listening {}", cfg.name, cfg.address);
    Ok(server)
}

/// Run the server.
/// Method __blocking__ current thread.
pub fn run<T>(service: T) -> Result<!, Error>
where
    T: 'static + std::marker::Sync + std::marker::Send + VMService,
{
    let _server = run_async(service)?;

    loop {
        thread::park();
    }
}

#[derive(Debug)]
pub struct ServerCfg {
    name: String,
    address: std::net::SocketAddr,
}

pub fn get_cfg_vars() -> Result<ServerCfg, Error> {
    let name = env::var("NAME").unwrap_or(String::from("unnamed vm-server"));
    let listen = env::var("LISTEN").expect("Expected LISTEN variable in format addr:port");
    Ok(ServerCfg {
        name,
        address: listen
            .to_socket_addrs()
            .map(|mut i| i.next().expect("Invalid address"))
            .expect("Invalid address"),
    })
}
