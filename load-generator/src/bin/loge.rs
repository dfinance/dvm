use http::Uri;
use clap::Clap;
use dvm_info::config::{InfoServiceConfig, MemoryOptions};
use load_generator::{ds, watcher};
use tokio::time::Duration;
use load_generator::dvm::Dvm;
use load_generator::tester::stat::statistic;
use load_generator::tester::run_load;
use std::sync::Arc;
use dvm_net::endpoint::Endpoint;
use std::str::FromStr;
use load_generator::info_service::InfoService;
use load_generator::log::CVSLog;

#[derive(Clap, Debug)]
#[clap(name = "loge.", version = "0.1.0")]
enum Loge {
    #[clap(about = "Attach to existing dvm.")]
    Attach {
        #[clap(name = "External dvm URI")]
        dvm: Uri,
        #[clap(
            name = "data_source_listen_address",
            default_value = "http://[::1]:5554",
            verbatim_doc_comment
        )]
        ds_address: Endpoint,
        #[clap(flatten)]
        test_info: TestInfo,
        #[clap(
            name = "info service URI",
            long = "info-service-uri",
            short = 'i',
            verbatim_doc_comment
        )]
        info_service_addr: Option<Uri>,
    },
    #[clap(about = "Run own dvm service.")]
    Start {
        path: String,
        #[clap(flatten)]
        info_service: InfoServiceConfig,
        #[clap(flatten)]
        memory_config: MemoryOptions,
        #[clap(default_value = "50051", long = "dvm_port")]
        dvm_port: u16,
        #[clap(default_value = "50052", long = "ds_port")]
        ds_port: u16,
        #[clap(default_value = "50053", long = "info_service_port")]
        info_service_port: u16,
        #[clap(flatten)]
        test_info: TestInfo,
    },
}

#[derive(Debug, Default, Clone, Clap)]
pub struct TestInfo {
    #[clap(default_value = "4", long = "threads_count", short = 't')]
    threads_count: usize,
    #[clap(default_value = "1h", long = "load_time")]
    load_time: String,
    #[clap(long = "csv_path")]
    csv_path: Option<String>,
    #[clap(default_value = "60", long = "log_interval")]
    log_interval: u64,
}

impl Loge {
    pub fn ds_endpoint(&self) -> Endpoint {
        match self {
            Loge::Attach { ds_address, .. } => ds_address.clone(),
            Loge::Start { ds_port, .. } => {
                Endpoint::from_str(&format!("http://127.0.0.1:{}", ds_port)).unwrap_or_else(|_| {
                    panic!(
                        "Failed to create endpoint with uri http://127.0.0.1:{}",
                        ds_port
                    )
                })
            }
        }
    }

    pub fn test_info(&self) -> &TestInfo {
        match self {
            Loge::Attach { test_info, .. } => test_info,
            Loge::Start { test_info, .. } => test_info,
        }
    }

    pub fn info_service(&self) -> Option<Uri> {
        match self {
            Loge::Attach {
                info_service_addr, ..
            } => info_service_addr.clone(),
            Loge::Start {
                info_service_port, ..
            } => Some(
                Uri::from_str(&format!("http://127.0.0.1:{}", info_service_port))
                    .expect("Valid url"),
            ),
        }
    }
}

#[tokio::main]
async fn main() {
    let options: Loge = Loge::parse();
    let ds = ds::start(options.ds_endpoint()).unwrap();
    let test_info = options.test_info().clone();
    let info_service = options
        .info_service()
        .map(|uri| InfoService::new(uri).unwrap());

    let logger = test_info.csv_path.map(|path| CVSLog::new(path).unwrap());

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
            info_service_port,
            ..
        } => Dvm::start(
            path,
            info_service,
            memory_config,
            dvm_port,
            ds_port,
            info_service_port,
        )
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
        Duration::from_secs(test_info.log_interval),
        info_service,
        logger,
    )
    .await
    .unwrap();
}
