use anyhow::{Result, Error};
use lazy_static::lazy_static;
use regex::Regex;
use libra::prelude::*;
use std::convert::TryFrom;
use chrono::{Utc, TimeZone};
use std::vec::IntoIter;

lazy_static! {
    static ref META_RE: Regex = Regex::new("\\#([A-Za-z0-9_]+)[:|=]?([A-Za-z0-9.:]*)").unwrap();
    static ref MODULE_RE: Regex = Regex::new("\\s*module\\s*([A-Za-z0-9_]+)").unwrap();
    static ref SCRIPT_RE: Regex = Regex::new("^\\s*script\\s*").unwrap();
    static ref FUNCTION_RE: Regex = Regex::new("\\s*fun\\s*([A-Za-z0-9_]+)").unwrap();
}

/// Test description;
#[derive(Debug, PartialEq)]
pub struct TestPipeline {
    pipeline: Vec<TestStep>,
}

impl TestPipeline {
    /// Create a new test pipeline.
    pub fn new(
        file_name: &str,
        content: &str,
        account_address: AccountAddress,
    ) -> Result<TestPipeline> {
        let mut pipeline = vec![];

        let mut meta: MetaState = Default::default();
        let mut look_for_fun = false;
        for (line_number, line) in content.lines().enumerate() {
            if look_for_fun {
                if let Some(capture) = FUNCTION_RE.captures(line) {
                    look_for_fun = false;
                    let function_name = capture[1].to_owned();
                    pipeline.push(TestStep::ExecuteScript((
                        TestMeta::take(&mut meta, account_address),
                        function_name,
                    )));
                }
            } else if let Some(capture) = META_RE.captures(line) {
                let meta_tag = MetaTag::try_from((&capture[1], &capture[2])).map_err(|err| {
                    anyhow!(
                        "Error:\"{}\" {} [{}:{}]",
                        err,
                        &capture[0],
                        file_name,
                        line_number
                    )
                })?;
                handle_meta_tag(&mut meta, meta_tag, file_name, line_number)?;
            } else if let Some(capture) = MODULE_RE.captures(line) {
                let module_name = capture[1].to_owned();
                pipeline.push(TestStep::PublishModule((
                    TestMeta::take(&mut meta, account_address),
                    module_name,
                )));
            } else if SCRIPT_RE.find(line).is_some() {
                look_for_fun = true;
            }
        }

        Ok(TestPipeline { pipeline })
    }

    /// Returns pipeline steps.
    pub fn steps(self) -> IntoIter<TestStep> {
        self.pipeline.into_iter()
    }
}

/// Handles the execution meta tag.
#[allow(unused_assignments)]
fn handle_meta_tag(
    meta: &mut MetaState,
    tag: MetaTag,
    file_name: &str,
    line_number: usize,
) -> Result<()> {
    match tag {
        MetaTag::Error(status) => match meta.expected_result {
            None => {
                meta.expected_result = Some(ExecutionResult::Error {
                    main_status: None,
                    additional_status: status,
                });
            }
            Some(ExecutionResult::Success) => {
                return Err(anyhow!(
                    "Expected result is already set. Error location [{}:{}]",
                    file_name,
                    line_number
                ));
            }
            Some(ExecutionResult::Error {
                main_status: _,
                mut additional_status,
            }) => {
                if additional_status == None {
                    additional_status = status;
                } else {
                    return Err(anyhow!(
                        "Expected result is already set. Error location [{}:{}]",
                        file_name,
                        line_number
                    ));
                }
            }
        },
        MetaTag::Address(addr) => {
            if meta.address == None {
                meta.address = Some(addr)
            } else {
                return Err(anyhow!(
                    "Account address is already set. Error location [{}:{}]",
                    file_name,
                    line_number
                ));
            }
        }
        MetaTag::Gas(g) => {
            if meta.gas == None {
                meta.gas = Some(g)
            } else {
                return Err(anyhow!(
                    "Max gas amount is already set. Error location [{}:{}]",
                    file_name,
                    line_number
                ));
            }
        }
        MetaTag::Status(status) => match meta.expected_result {
            None => {
                meta.expected_result = Some(ExecutionResult::Error {
                    main_status: Some(status),
                    additional_status: None,
                });
            }
            Some(ExecutionResult::Success) => {
                return Err(anyhow!(
                    "Expected result is already set. Error location [{}:{}]",
                    file_name,
                    line_number
                ));
            }
            Some(ExecutionResult::Error {
                mut main_status,
                additional_status: _,
            }) => {
                if main_status == None {
                    main_status = Some(status);
                } else {
                    return Err(anyhow!(
                        "Expected result is already set. Error location [{}:{}]",
                        file_name,
                        line_number
                    ));
                }
            }
        },
        MetaTag::Time(time) => {
            if meta.time.is_none() {
                meta.time = Some(time);
            } else {
                return Err(anyhow!(
                    "Time is already set. Error location [{}:{}]",
                    file_name,
                    line_number
                ));
            }
        }
        MetaTag::Block(block) => {
            if meta.block.is_none() {
                meta.block = Some(block);
            } else {
                return Err(anyhow!(
                    "Block is already set. Error location [{}:{}]",
                    file_name,
                    line_number
                ));
            }
        }
        MetaTag::Price(price) => {
            if meta.oracle_price_list.is_none() {
                meta.oracle_price_list = Some(vec![]);
            }
            if let Some(list) = meta.oracle_price_list.as_mut() {
                list.push(price);
            }
        }
        MetaTag::Success => {
            if meta.expected_result.is_none() {
                meta.expected_result = Some(ExecutionResult::Success)
            } else {
                return Err(anyhow!(
                    "Expected result is already set. Error location [{}:{}]",
                    file_name,
                    line_number
                ));
            }
        }
    }
    Ok(())
}

#[derive(Debug, PartialEq)]
pub enum TestStep {
    PublishModule((TestMeta, String)),
    ExecuteScript((TestMeta, String)),
}

impl TestStep {
    pub fn meta(&self) -> &TestMeta {
        match self {
            TestStep::PublishModule((meta, _)) => meta,
            TestStep::ExecuteScript((meta, _)) => meta,
        }
    }

    pub fn unit(&self) -> &str {
        match self {
            TestStep::PublishModule((_, unit)) => unit,
            TestStep::ExecuteScript((_, unit)) => unit,
        }
    }
}

/// Test metadata builder.
#[derive(Debug, PartialEq, Default)]
struct MetaState {
    address: Option<AccountAddress>,
    gas: Option<u64>,
    expected_result: Option<ExecutionResult>,
    block: Option<u64>,
    time: Option<u64>,
    oracle_price_list: Option<Vec<(String, u64)>>,
}

/// Test metadata.
#[derive(Debug, PartialEq)]
pub struct TestMeta {
    pub address: AccountAddress,
    pub gas: u64,
    pub expected_result: ExecutionResult,
    pub block: u64,
    pub time: u64,
    pub oracle_price_list: Vec<(String, u64)>,
}

impl TestMeta {
    /// Returns test metadata.
    fn take(meta: &mut MetaState, address: AccountAddress) -> TestMeta {
        TestMeta {
            address: meta.address.take().unwrap_or_else(|| address),
            gas: meta.gas.take().unwrap_or_else(u64::max_value),
            expected_result: meta
                .expected_result
                .take()
                .unwrap_or_else(|| ExecutionResult::Success),
            block: meta.block.take().unwrap_or_else(|| 100),
            time: meta
                .time
                .take()
                .unwrap_or_else(|| Utc::now().timestamp() as u64),
            oracle_price_list: meta.oracle_price_list.take().unwrap_or_else(Vec::new),
        }
    }
}

/// Transaction execution result.
#[derive(Debug, PartialEq)]
pub enum ExecutionResult {
    Success,
    Error {
        main_status: Option<u64>,
        additional_status: Option<u64>,
    },
}

/// Test meta tag.
#[derive(Debug, PartialEq)]
pub enum MetaTag {
    /// Success result.
    Success,
    /// Assert error code.
    Error(Option<u64>),
    /// Execution account address.
    Address(AccountAddress),
    /// Max gas.
    Gas(u64),
    /// Error main status.
    Status(u64),
    /// Time oracle value. Format dd.MM.yyyyTHH:mm:ss.
    Time(u64),
    /// Block number.
    Block(u64),
    /// Oracle price.
    Price((String, u64)),
}

impl TryFrom<(&str, &str)> for MetaTag {
    type Error = Error;

    fn try_from((key, value): (&str, &str)) -> Result<Self, Error> {
        let tag = key.to_lowercase();
        match tag.as_str() {
            "error" => {
                if value.is_empty() {
                    Ok(MetaTag::Error(None))
                } else {
                    Ok(MetaTag::Error(Some(value.parse()?)))
                }
            }
            "address" => Ok(MetaTag::Address(AccountAddress::from_hex_literal(value)?)),
            "gas" => Ok(MetaTag::Gas(value.parse()?)),
            "status" => Ok(MetaTag::Status(value.parse()?)),
            "block" => Ok(MetaTag::Block(value.parse()?)),
            "time" => {
                let date_time = Utc.datetime_from_str(value, "%d.%m.%YT%H:%M:%S")?;
                Ok(MetaTag::Time(date_time.timestamp() as u64))
            }
            "success" => Ok(MetaTag::Success),
            _ => Ok(MetaTag::Price((key.to_owned(), value.parse()?))),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use libra::prelude::*;
    use crate::test_suite::pipeline::{MetaTag, TestPipeline, TestStep, TestMeta, ExecutionResult};
    use std::convert::TryFrom;

    #[test]
    fn test_parse_pipeline() {
        let content = "
            //#time:24.06.2020T16:50:0
            module Test1 {}
            //#time:24.06.2020T16:50:0
            script {
                fun main1() {
                }
            }
            //#time:24.06.2020T16:50:0
            //#error:100
            script {
                fun main2() {
                }
            }
            //#time:24.06.2020T16:50:0
            //#address=0x02
            module Test2 {}

            //#time:24.06.2020T16:50:1
            //#error:100
            //#gas=100
            //#address=0x0202
            //#Status=4001
            script {
                fun main3() {
                }
            }
            //#success:
            //#block=1
            //#time:24.06.2020T06:00:0
            script {
                fun main4() {}
            }
            //#time:24.06.2020T16:51:1
            //#error
            //#usd_btc:80
            script {
                fun main5() {}
            }
       ";
        let pipeline = TestPipeline::new("pipeline", content, CORE_CODE_ADDRESS).unwrap();
        assert_eq!(
            pipeline,
            TestPipeline {
                pipeline: vec![
                    TestStep::PublishModule((
                        TestMeta {
                            address: CORE_CODE_ADDRESS,
                            gas: 18446744073709551615,
                            expected_result: ExecutionResult::Success,
                            block: 100,
                            time: 1593017400,
                            oracle_price_list: vec![],
                        },
                        "Test1".to_owned()
                    )),
                    TestStep::ExecuteScript((
                        TestMeta {
                            address: CORE_CODE_ADDRESS,
                            gas: 18446744073709551615,
                            expected_result: ExecutionResult::Success,
                            block: 100,
                            time: 1593017400,
                            oracle_price_list: vec![],
                        },
                        "main1".to_owned()
                    )),
                    TestStep::ExecuteScript((
                        TestMeta {
                            address: CORE_CODE_ADDRESS,
                            gas: 18446744073709551615,
                            expected_result: ExecutionResult::Error {
                                main_status: None,
                                additional_status: Some(100),
                            },
                            block: 100,
                            time: 1593017400,
                            oracle_price_list: vec![],
                        },
                        "main2".to_owned()
                    )),
                    TestStep::PublishModule((
                        TestMeta {
                            address: AccountAddress::from_hex_literal("0x02").unwrap(),
                            gas: 18446744073709551615,
                            expected_result: ExecutionResult::Success,
                            block: 100,
                            time: 1593017400,
                            oracle_price_list: vec![],
                        },
                        "Test2".to_owned()
                    )),
                    TestStep::ExecuteScript((
                        TestMeta {
                            address: AccountAddress::from_hex_literal("0x0202").unwrap(),
                            gas: 100,
                            expected_result: ExecutionResult::Error {
                                main_status: None,
                                additional_status: Some(100),
                            },
                            block: 100,
                            time: 1593017401,
                            oracle_price_list: vec![],
                        },
                        "main3".to_owned()
                    )),
                    TestStep::ExecuteScript((
                        TestMeta {
                            address: CORE_CODE_ADDRESS,
                            gas: 18446744073709551615,
                            expected_result: ExecutionResult::Success,
                            block: 1,
                            time: 1592978400,
                            oracle_price_list: vec![],
                        },
                        "main4".to_owned()
                    )),
                    TestStep::ExecuteScript((
                        TestMeta {
                            address: CORE_CODE_ADDRESS,
                            gas: 18446744073709551615,
                            expected_result: ExecutionResult::Error {
                                main_status: None,
                                additional_status: None,
                            },
                            block: 100,
                            time: 1593017461,
                            oracle_price_list: vec![("usd_btc".to_owned(), 80)],
                        },
                        "main5".to_owned()
                    )),
                ]
            }
        );
    }

    #[test]
    fn test_meta_tag() {
        assert_eq!(
            MetaTag::Error(Some(10)),
            MetaTag::try_from(("error", "10")).unwrap()
        );
        assert_eq!(
            MetaTag::Error(None),
            MetaTag::try_from(("error", "")).unwrap()
        );
        assert_eq!(
            MetaTag::Gas(50000),
            MetaTag::try_from(("gas", "50000")).unwrap()
        );
        assert_eq!(
            MetaTag::Status(400),
            MetaTag::try_from(("status", "400")).unwrap()
        );
        assert_eq!(
            MetaTag::Address(AccountAddress::from_hex_literal("0x3").unwrap()),
            MetaTag::try_from(("address", "0x3")).unwrap()
        );
        assert_eq!(
            MetaTag::Block(200),
            MetaTag::try_from(("block", "200")).unwrap()
        );
        assert_eq!(
            MetaTag::Time(1592978400),
            MetaTag::try_from(("time", "24.06.2020T06:00:0")).unwrap()
        );
        assert_eq!(
            MetaTag::Price(("usd_btc".to_owned(), 100)),
            MetaTag::try_from(("usd_btc", "100")).unwrap()
        );
    }
}
