use std::net::{SocketAddr, ToSocketAddrs};

#[derive(Debug)]
pub struct Cfg<T> {
    pub name: String,
    pub address: T,
}

impl<T: ToSocketAddrs> Cfg<T> {
    pub fn into_sock_addr(self) -> Result<Cfg<SocketAddr>, std::io::Error> {
        Ok(Cfg {
            name: self.name,
            address: self
                .address
                .to_socket_addrs()
                .map(|mut i| i.next().expect("Invalid address"))?,
        })
    }
}

pub mod env {
    use super::*;
    use std::env;

    pub fn get_cfg_vars() -> Cfg<String> {
        let name = env::var("NAME").unwrap_or_else(|_| String::from("unnamed"));
        let address = env::var("LISTEN").expect("Expected LISTEN variable in format addr:port");
        Cfg { name, address }
    }
}
