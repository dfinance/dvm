use std::sync::mpsc::{channel, Receiver, Sender};
use anyhow::Error;
use std::{thread, fmt};
use std::thread::JoinHandle;

pub struct StatCollector {
    _handler: JoinHandle<()>,
    sender: Sender<Operation>,
    receiver: Receiver<Statistic>,
}

impl StatCollector {
    pub fn new(receiver: Receiver<Operation>, sender: Sender<Operation>) -> StatCollector {
        let (stat_sender, stat_receiver) = channel();

        let handler = thread::spawn(move || {
            let mut statistic = Statistic::new();
            for stat in receiver.iter() {
                match stat {
                    Operation::Write(Stat::CompileScript(time)) => {
                        statistic.compile_script_avg.update(time)
                    }
                    Operation::Write(Stat::PublishModule(time)) => {
                        statistic.publish_module_avg.update(time)
                    }
                    Operation::Write(Stat::ExecuteScript(time)) => {
                        statistic.execute_script_avg.update(time)
                    }
                    Operation::Write(Stat::CompileModule(time)) => {
                        statistic.compile_module_avg.update(time)
                    }
                    Operation::Read => {
                        stat_sender.send(statistic.take()).unwrap();
                    }
                }
            }
        });

        StatCollector {
            _handler: handler,
            sender,
            receiver: stat_receiver,
        }
    }

    pub fn statistics(&self) -> Result<Statistic, Error> {
        self.sender.send(Operation::Read)?;
        Ok(self.receiver.recv()?)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Average {
    pub total_time: u128,
    pub quadratic_total_time: u128,
    pub items: u64,
}

impl Average {
    pub fn take(&mut self) -> Average {
        let avg = Average {
            total_time: self.total_time,
            quadratic_total_time: self.quadratic_total_time,
            items: self.items,
        };

        self.total_time = 0;
        self.quadratic_total_time = 0;
        self.items = 0;

        avg
    }

    pub fn update(&mut self, time: u128) {
        self.total_time += time;
        self.quadratic_total_time += time * time;
        self.items += 1;
    }

    pub fn avg(&self) -> u128 {
        if self.total_time == 0 {
            0
        } else {
            self.total_time / self.items as u128
        }
    }

    pub fn sd(&self) -> f64 {
        if self.quadratic_total_time == 0 {
            0.0
        } else {
            let avg = self.avg();
            let quadratic_time = self.quadratic_total_time / self.items as u128;
            ((quadratic_time - (avg * avg)) as f64).sqrt()
        }
    }
}

impl fmt::Display for Average {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "avg[{}±{:.2}]", self.avg(), self.sd())
    }
}

#[derive(Debug, Clone, Default)]
pub struct Statistic {
    pub compile_script_avg: Average,
    pub publish_module_avg: Average,
    pub execute_script_avg: Average,
    pub compile_module_avg: Average,
}

impl Statistic {
    pub fn new() -> Statistic {
        Statistic {
            compile_script_avg: Average::default(),
            publish_module_avg: Average::default(),
            execute_script_avg: Average::default(),
            compile_module_avg: Average::default(),
        }
    }

    pub fn take(&mut self) -> Statistic {
        Statistic {
            compile_script_avg: self.compile_script_avg.take(),
            publish_module_avg: self.publish_module_avg.take(),
            execute_script_avg: self.execute_script_avg.take(),
            compile_module_avg: self.compile_module_avg.take(),
        }
    }
}

impl fmt::Display for Statistic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "compile_script:{}; execute_script:{}; compile_module:{}, publish_module:{}",
            self.compile_script_avg,
            self.execute_script_avg,
            self.compile_module_avg,
            self.publish_module_avg
        )
    }
}

#[derive(Debug, Clone)]
pub enum Operation {
    Read,
    Write(Stat),
}

#[derive(Debug, Clone)]
pub enum Stat {
    CompileScript(u128),
    CompileModule(u128),
    PublishModule(u128),
    ExecuteScript(u128),
}

#[derive(Debug, Clone)]
pub struct StatWriter {
    sender: Sender<Operation>,
}

impl StatWriter {
    pub fn store(&self, stat: Stat) -> Result<(), Error> {
        Ok(self.sender.send(Operation::Write(stat))?)
    }
}

pub fn statistic() -> (StatCollector, StatWriter) {
    let (sender, receiver) = channel();
    (
        StatCollector::new(receiver, sender.clone()),
        StatWriter { sender },
    )
}
