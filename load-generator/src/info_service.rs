use http::Uri;
use anyhow::Error;
use std::str::FromStr;
use std::fmt;
use std::fmt::Formatter;

pub struct InfoService {
    url: String,
}

impl InfoService {
    pub fn new(uri: Uri) -> Result<InfoService, Error> {
        Ok(InfoService {
            url: format!("{}metrics", uri.to_string()),
        })
    }

    pub async fn load_info(&mut self) -> Result<SystemInfo, Error> {
        Ok(reqwest::get(&self.url)
            .await?
            .text()
            .await?
            .lines()
            .filter(|l| !l.starts_with('#'))
            .fold(
                SystemInfo {
                    cpu_usage: None,
                    memory: None,
                },
                |mut info, line| {
                    if !info.filled() {
                        if line.starts_with("dvm_sys_info_cpu_usage") {
                            if let Some(value) = line.split(' ').nth(1) {
                                info.cpu_usage = f32::from_str(value).ok();
                            }
                        } else if line.starts_with("dvm_sys_info_memory") {
                            if let Some(value) = line.split(' ').nth(1) {
                                info.memory = u64::from_str(value).ok();
                            }
                        }
                    }
                    info
                },
            ))
    }
}

#[derive(Default)]
pub struct SystemInfo {
    /// Total CPU usage.
    pub cpu_usage: Option<f32>,
    /// Memory usage for the process (in kB).
    pub memory: Option<u64>,
}

impl SystemInfo {
    pub fn filled(&self) -> bool {
        self.cpu_usage.is_some() && self.memory.is_some()
    }
}

impl fmt::Display for SystemInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(cpu_usage) = self.cpu_usage {
            write!(f, "cpu usage[{:.2}]; ", cpu_usage)?
        } else {
            write!(f, "cpu usage[?]; ")?
        }

        if let Some(memory) = self.memory {
            if memory > 1024 * 1024 * 1024 {
                write!(f, "memory[{}GiB];", memory / 1024 / 1024 / 1024)?
            } else if memory > 1024 * 1024 {
                write!(f, "memory[{}MiB];", memory / 1024 / 1024)?
            } else if memory > 1024 {
                write!(f, "memory[{}KiB];", memory / 1024)?
            } else {
                write!(f, "memory[{}];", memory)?
            }
        } else {
            write!(f, "memory[?];")?
        }

        Ok(())
    }
}
