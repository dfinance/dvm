use http::Uri;
use clap::Clap;
use dvm_info::config::{InfoServiceConfig, MemoryOptions};
use load_generator::{ds, watcher};
use tokio::time::Duration;
use load_generator::dvm::Dvm;
use load_generator::tester::stat::statistic;
use load_generator::tester::run_load;
use std::sync::Arc;

#[derive(Clap, Debug)]
#[clap(name = "loge.", version = "0.1.0")]
enum Loge {
    #[clap(about = "Attach to existing dvm.")]
    Attach {
        #[clap(name = "External dvm URI")]
        dvm: Uri,
        #[clap(default_value = "5554", long = "ds_port")]
        ds_port: u16,
        #[clap(flatten)]
        test_info: TestInfo,
    },
    #[clap(about = "Run own dvm service.")]
    Start {
        path: String,
        #[clap(flatten)]
        info_service: InfoServiceConfig,
        #[clap(flatten)]
        memory_config: MemoryOptions,
        #[clap(default_value = "5555", long = "dvm_port")]
        dvm_port: u16,
        #[clap(default_value = "5554", long = "ds_port")]
        ds_port: u16,

        #[clap(flatten)]
        test_info: TestInfo,
    },
}

#[derive(Debug, Default, Clone, Clap)]
pub struct TestInfo {
    #[clap(default_value = "1", long = "threads_count", short = 't')]
    threads_count: usize,
    #[clap(default_value = "1h", long = "load_time")]
    load_time: String,
}

impl Loge {
    pub fn ds_port(&self) -> u16 {
        match self {
            Loge::Attach { ds_port, .. } => *ds_port,
            Loge::Start { ds_port, .. } => *ds_port,
        }
    }

    pub fn test_info(&self) -> &TestInfo {
        match self {
            Loge::Attach { test_info, .. } => test_info,
            Loge::Start { test_info, .. } => test_info,
        }
    }
}

#[tokio::main]
async fn main() {
    let options: Loge = Loge::parse();
    let ds = ds::start(options.ds_port()).unwrap();
    let test_info = options.test_info().clone();

    let dvm = match options {
        Loge::Attach { dvm, .. } => {
            Dvm::connect(dvm).map_err(|err| format!("Failed to connect to external dvm: {:?}", err))
        }
        Loge::Start {
            path,
            info_service,
            memory_config,
            dvm_port,
            ds_port,
            ..
        } => Dvm::start(path, info_service, memory_config, dvm_port, ds_port)
            .map_err(|err| format!("Failed to start dvm process: {:?}", err)),
    }
    .unwrap_or_else(|err| {
        println!("{}", err);
        std::process::exit(1)
    });

    dvm.wait_for().await;
    let dvm = Arc::new(dvm);

    let (stat_collector, stat_writer) = statistic();
    let handler = run_load(dvm.clone(), test_info.threads_count, stat_writer, ds);

    watcher::watch(
        handler,
        stat_collector,
        test_info.load_time.parse().unwrap(),
        Duration::from_secs(60),
    )
    .await
    .unwrap();
}
