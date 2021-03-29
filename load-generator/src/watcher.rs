use crate::tester::LoadHandler;
use std::str::FromStr;
use anyhow::{Error, anyhow};
use crate::tester::stat::{StatCollector, Statistic};
use tokio::time::{Duration, Instant, delay_for};
use std::fmt;
use std::io::{stdout, Write};
use crate::info_service::{InfoService, SystemInfo};
use crate::log::Log;

pub async fn watch(
    handler: LoadHandler,
    stat_collector: StatCollector,
    load_time: TimeInterval,
    update_interval: Duration,
    mut info_service: Option<InfoService>,
    mut logger: Option<impl Log>,
) -> Result<(), Error> {
    if !handler.is_run() {
        return Err(anyhow!("Failed to start load"));
    }

    println!("Load started.");

    let mut timer = LoadTimer::new(load_time);
    timer.start();

    let mut total_iterations = 0;
    loop {
        delay_for(update_interval).await;
        let statistics = stat_collector.statistics()?;
        total_iterations += statistics.publish_module_avg.items;
        let left = timer.time_left();

        if !handler.is_run() {
            return Err(anyhow!("An error occurred during stress tests."));
        }

        let sys_info = if let Some(srv) = &mut info_service {
            srv.load_info().await?
        } else {
            Default::default()
        };

        if let Some(logger) = &mut logger {
            logger.log(total_iterations, &statistics, &sys_info)?;
        }

        print_status(&left, total_iterations, &statistics, sys_info)?;

        if left.is_zero() {
            println!("Time is over");
            break;
        }
    }

    handler.shutdown();
    Ok(())
}

#[derive(Debug, Clone)]
pub struct LoadTimer {
    duration: Duration,
    start_at: Option<Instant>,
}

impl LoadTimer {
    pub fn new(interval: TimeInterval) -> LoadTimer {
        LoadTimer {
            duration: interval.duration,
            start_at: None,
        }
    }

    pub fn start(&mut self) {
        self.start_at = Some(Instant::now());
    }

    pub fn time_left(&self) -> TimeInterval {
        if let Some(start_at) = &self.start_at {
            let elapsed = start_at.elapsed();
            if elapsed > self.duration {
                TimeInterval::zero_time()
            } else {
                TimeInterval::with_duration(self.duration - elapsed)
            }
        } else {
            TimeInterval::zero_time()
        }
    }
}

impl From<LoadTimer> for Duration {
    fn from(lt: LoadTimer) -> Self {
        lt.duration
    }
}

impl fmt::Display for LoadTimer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.duration.as_secs())
    }
}

pub struct TimeInterval {
    duration: Duration,
}

impl TimeInterval {
    pub fn zero_time() -> TimeInterval {
        TimeInterval {
            duration: Duration::from_secs(0),
        }
    }

    pub fn with_duration(duration: Duration) -> TimeInterval {
        TimeInterval { duration }
    }

    pub fn is_zero(&self) -> bool {
        self.duration.as_secs() == 0
    }
}

impl FromStr for TimeInterval {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut duration = Duration::from_secs(0);
        let re = regex::Regex::new(r"\d{1,}[dhms]")?;
        for part in re.find_iter(s) {
            let units: u64 = s[part.start()..part.end() - 1].parse()?;
            duration += match &s[part.end() - 1..part.end()] {
                "d" => Duration::from_secs(units * 60 * 60 * 24),
                "h" => Duration::from_secs(units * 60 * 60),
                "m" => Duration::from_secs(units * 60),
                "s" => Duration::from_secs(units),
                _ => Duration::from_secs(0),
            };
        }
        Ok(TimeInterval { duration })
    }
}

impl fmt::Display for TimeInterval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let total_minutes = self.duration.as_secs() / 60;
        let minutes = total_minutes % 60;

        let total_hours = (total_minutes - minutes) / 60;
        let hours = total_hours % 24;
        let days = total_hours / 24;
        if days > 0 {
            write!(f, "{}d", days)?;
        }

        if hours > 0 {
            write!(f, "{}h", hours)?;
        }

        if minutes > 0 {
            write!(f, "{}m", minutes)
        } else {
            Ok(())
        }
    }
}

fn print_status(
    left: &TimeInterval,
    total_iterations: u64,
    stat: &Statistic,
    sys_info: SystemInfo,
) -> Result<(), Error> {
    print!(
        "\rTime left:{}; number of iterations:{}; statistics:{}; {}",
        left, total_iterations, stat, sys_info
    );
    stdout().flush()?;
    Ok(())
}
