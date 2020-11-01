use anyhow::Error;
use crate::tester::stat::Statistic;
use crate::info_service::SystemInfo;
use std::path::Path;
use std::fs::File;
use chrono::Local;

pub trait Log {
    fn log(
        &mut self,
        total_iterations: u64,
        stat: &Statistic,
        sys_info: &SystemInfo,
    ) -> Result<(), Error>;
}

pub struct CVSLog {
    wtr: csv::Writer<File>,
}

impl CVSLog {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<CVSLog, Error> {
        let mut wtr = csv::Writer::from_path(path)?;

        wtr.write_record(&[
            "DateTime",
            "TotalIterations",
            "CompileScriptAvg",
            "CompileScriptSD",
            "PublishModuleAvg",
            "PublishModuleSD",
            "ExecuteScriptAvg",
            "ExecuteScriptSD",
            "CompileModuleAvg",
            "CompileModuleSD",
            "MemoryUsage",
            "CPUUtilization",
        ])?;
        wtr.flush()?;
        Ok(CVSLog { wtr })
    }
}

impl Log for CVSLog {
    fn log(
        &mut self,
        total_iterations: u64,
        stat: &Statistic,
        sys_info: &SystemInfo,
    ) -> Result<(), Error> {
        self.wtr.serialize((
            Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            total_iterations,
            stat.compile_script_avg.avg(),
            stat.compile_script_avg.sd(),
            stat.publish_module_avg.avg(),
            stat.publish_module_avg.sd(),
            stat.execute_script_avg.avg(),
            stat.execute_script_avg.sd(),
            stat.compile_module_avg.avg(),
            stat.compile_module_avg.sd(),
            sys_info.memory,
            sys_info.cpu_usage,
        ))?;
        self.wtr.flush().map_err(|err| err.into())
    }
}
