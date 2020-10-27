use dvm_info::config::{InfoServiceConfig, MemoryOptions};

pub trait IntoArgs {
    fn into_args(self) -> Vec<String>;
}

impl IntoArgs for InfoServiceConfig {
    fn into_args(self) -> Vec<String> {
        let mut args = Vec::with_capacity(8);

        if let Some(info_service_addr) = &self.info_service_addr {
            args.push("--info-service-addr".to_owned());
            args.push(info_service_addr.to_string());
        }

        if let Some(dvm_self_check_addr) = &self.dvm_self_check_addr {
            args.push("--dvm-self-check-addr".to_owned());
            args.push(dvm_self_check_addr.to_string());
        }

        args.push("--metric-update-interval".to_owned());
        args.push(5.to_string());

        args.push("--heartbeat-interval-max".to_owned());
        args.push(5.to_string());

        args.push("--heartbeat-pressure".to_owned());
        args.push(4.to_string());

        args
    }
}

impl IntoArgs for MemoryOptions {
    fn into_args(self) -> Vec<String> {
        let mut args = Vec::with_capacity(8);

        args.push("--module_cache_size".to_owned());
        args.push(102400.to_string());

        args.push("--memory_check_period".to_owned());
        args.push(10000.to_string());

        args.push("--dvm_cache_size".to_owned());
        args.push(102400.to_string());

        args
    }
}
