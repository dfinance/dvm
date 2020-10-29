mod args;
pub mod client;

use http::Uri;
use dvm_info::config::{InfoServiceConfig, MemoryOptions};
use std::path::Path;
use std::process::{Command, Stdio, Child};
use std::{env, fs};
use anyhow::Error;
use crate::dvm::args::IntoArgs;
use crate::dvm::client::Client;
use tokio::time::{delay_for, Duration};
use libra::account::CORE_CODE_ADDRESS;

#[derive(Debug)]
pub enum Dvm {
    Own { process: Child, uri: Uri },
    External(Uri),
}

impl Dvm {
    pub fn connect(uri: Uri) -> Result<Dvm, Error> {
        Ok(Dvm::External(uri))
    }

    pub fn start<P: AsRef<Path>>(
        path: P,
        info_service: InfoServiceConfig,
        memory_config: MemoryOptions,
        dvm_port: u16,
        ds_port: u16,
    ) -> Result<Dvm, Error> {
        let info_service = info_service.into_args();
        let memory_config = memory_config.into_args();
        println!(
            "Run dvm process:[{} http://0.0.0.0:{} http://127.0.0.1:{} {} {}]",
            path.as_ref().to_str().unwrap_or("dvm"),
            dvm_port,
            ds_port,
            info_service.join(" "),
            memory_config.join(" ")
        );

        let process = Command::new(&fs::canonicalize(env::current_dir()?.join(path))?)
            .arg(format!("http://0.0.0.0:{}", dvm_port))
            .arg(format!("http://127.0.0.1:{}", ds_port))
            .args(info_service)
            .args(memory_config)
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .spawn()?;

        println!("Dvm pid:{}", process.id());
        Ok(Dvm::Own {
            process,
            uri: format!("http://127.0.0.1:{}", dvm_port).parse().unwrap(),
        })
    }

    pub async fn wait_for(&self) {
        println!("Wait for dvm.");
        loop {
            let uri = match self {
                Dvm::Own { uri, .. } => uri,
                Dvm::External(uri) => uri,
            };

            if let Ok(mut cl) = Client::new(dbg!(uri.clone())).await {
                if cl.compile("module A {}", CORE_CODE_ADDRESS).await.is_ok() {
                    println!("Connected");
                    break;
                }
            }
            delay_for(Duration::from_secs(2)).await;
        }
    }

    pub async fn bind_client(&self) -> Result<Client, Error> {
        match self {
            Dvm::Own { uri, .. } => Client::new(uri.clone()).await,
            Dvm::External(uri) => Client::new(uri.clone()).await,
        }
    }
}

impl Drop for Dvm {
    fn drop(&mut self) {
        if let Dvm::Own { process, .. } = self {
            println!("Dropping Dvm");
            if let Err(err) = process.kill() {
                println!("Failed to kill dvm process:[pid={}] {}", process.id(), err);
            }
        }
    }
}
