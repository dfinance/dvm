use crate::tester::stat::{StatWriter, Stat};
use crate::dvm::client::Client;
use crate::ds::InMemoryDataSource;
use anyhow::{Error, anyhow};
use rand::random;
use std::time::Instant;
use libra::account::AccountAddress;
use crate::tester::mv_template::{module, store_script, load_script};
use libra::result::StatusCode;
use dvm_net::api::grpc::vm_grpc::VmArgs;
use dvm_net::api::grpc::types::VmTypeTag;
use byteorder::{LittleEndian, ByteOrder};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::task::JoinHandle;
use crate::dvm::Dvm;

const MAX_GAS: u64 = 400_000;

pub struct Pipeline {
    stat: StatWriter,
    client: Bind,
    ds: InMemoryDataSource,
}

impl Pipeline {
    pub fn new(stat: StatWriter, dvm: Arc<Dvm>, ds: InMemoryDataSource) -> Pipeline {
        Pipeline {
            stat,
            client: Bind::Dvm(dvm),
            ds,
        }
    }

    pub async fn execute(&mut self) -> Result<(), Error> {
        self.ensure_bind().await?;
        let client = self.client.as_mut();
        let name = random::<u128>().to_string();
        let address = AccountAddress::random();

        let module = module(&name, &address);
        let instant = Instant::now();
        let module = client.compile(&module, address).await?;
        self.stat
            .store(Stat::CompileModule(instant.elapsed().as_millis()))?;

        let instant = Instant::now();
        let publish_result = client.publish(module, MAX_GAS, 1, address).await?;
        self.stat
            .store(Stat::PublishModule(instant.elapsed().as_millis()))?;
        if publish_result.status != StatusCode::EXECUTED {
            return Err(anyhow!(
                "Unexpected publish module result:{:?}",
                publish_result.status
            ));
        }
        self.ds.store_write_set(publish_result.ws);

        let store_script = store_script(&name, &address);
        let instant = Instant::now();
        let store_script = client.compile(&store_script, address).await?;
        self.stat
            .store(Stat::CompileScript(instant.elapsed().as_millis()))?;

        let mut buf = vec![0; 8];
        LittleEndian::write_u64(&mut buf, random());
        let mut args = vec![VmArgs {
            r#type: VmTypeTag::U64 as i32,
            value: buf,
        }];

        let instant = Instant::now();
        let execution_result = client
            .execute(
                store_script,
                MAX_GAS,
                1,
                vec![address],
                args.clone(),
                vec![],
            )
            .await?;
        self.stat
            .store(Stat::ExecuteScript(instant.elapsed().as_millis()))?;
        if execution_result.status != StatusCode::EXECUTED {
            return Err(anyhow!(
                "Unexpected  execution result:{:?}",
                execution_result.status
            ));
        }
        self.ds.store_write_set(execution_result.ws);

        let load_script = load_script(&name, &address);
        let instant = Instant::now();
        let load_script = client.compile(&load_script, address).await?;
        self.stat
            .store(Stat::CompileScript(instant.elapsed().as_millis()))?;

        args.push(VmArgs {
            r#type: VmTypeTag::Address as i32,
            value: address.to_vec(),
        });
        let instant = Instant::now();
        let execution_result = client
            .execute(load_script, MAX_GAS, 1, vec![address], args, vec![])
            .await?;
        self.stat
            .store(Stat::ExecuteScript(instant.elapsed().as_millis()))?;
        if execution_result.status != StatusCode::EXECUTED {
            return Err(anyhow!(
                "Unexpected  execution result:{:?}",
                execution_result.status
            ));
        }
        Ok(())
    }

    async fn ensure_bind(&mut self) -> Result<(), Error> {
        if let Bind::Dvm(dvm) = &self.client {
            self.client = Bind::Client(dvm.bind_client().await?)
        }
        Ok(())
    }
}

pub fn perform(mut pipeline: Pipeline) -> Handler {
    println!("Starting load worker.");
    let is_run = Arc::new(AtomicBool::new(true));

    let is_run_clone = is_run.clone();
    let handler = tokio::task::spawn(async move {
        while is_run_clone.load(Ordering::Relaxed) {
            if let Err(err) = pipeline.execute().await {
                println!("Stop pipeline with error: {:?}", err);
                is_run_clone.store(false, Ordering::Relaxed);
            }
        }
    });

    Handler {
        is_run,
        _inner: handler,
    }
}

pub struct Handler {
    is_run: Arc<AtomicBool>,
    _inner: JoinHandle<()>,
}

impl Handler {
    pub fn is_run(&self) -> bool {
        self.is_run.load(Ordering::Relaxed)
    }

    pub fn stop(&self) {
        self.is_run.store(false, Ordering::Relaxed);
    }
}

impl Drop for Handler {
    fn drop(&mut self) {
        self.is_run.store(false, Ordering::Relaxed);
    }
}

pub enum Bind {
    Dvm(Arc<Dvm>),
    Client(Client),
}

impl AsMut<Client> for Bind {
    fn as_mut(&mut self) -> &mut Client {
        match self {
            Bind::Dvm(_) => panic!("Deref unbonded client."),
            Bind::Client(client) => client,
        }
    }
}
