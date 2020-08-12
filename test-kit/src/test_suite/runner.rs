use runtime::{
    move_vm::{Dvm, ExecutionMeta, Script, VmResult},
    resources::{
        block_metadata, time_metadata, oracle_metadata, BlockMetadata, Price, CurrentTimestamp,
    },
};
use data_source::MockDataSource;
use libra::prelude::*;
use libra::lcs;
use compiler::Compiler;
use anyhow::Result;
use crate::test_suite::pipeline::{TestPipeline, TestStep, TestMeta, ExecutionResult};
use std::collections::HashMap;

/// Test pipeline state.
pub struct TestState {
    compiler: Compiler<MockDataSource>,
    ds: MockDataSource,
    test_name: String,
    test_content: String,
}

impl TestState {
    /// Create a new TestState.
    pub fn new(stdlib: WriteSet, test_name: String, test_content: String) -> TestState {
        let ds = MockDataSource::with_write_set(stdlib);
        TestState {
            compiler: Compiler::new(ds.clone()),
            ds,
            test_name,
            test_content,
        }
    }

    /// Performs test pipeline.
    pub fn perform(self) -> Result<()> {
        let pipeline = TestPipeline::new(
            &self.test_name,
            &self.test_content,
            AccountAddress::random(),
        )?;
        let mut source_map = HashMap::new();

        source_map.insert("Source".to_owned(), self.test_content);
        let byte_code_map = self.compiler.compile_source_map(source_map, None)?;

        for step in pipeline.steps() {
            Self::perform_step(&self.ds, &step, &byte_code_map)
                .map_err(|err| anyhow!("Step:[{}] - {} ", step.unit(), err))?;
        }
        Ok(())
    }

    /// Performs test step.
    fn perform_step(
        main_ds: &MockDataSource,
        step: &TestStep,
        byte_code_map: &HashMap<String, Vec<u8>>,
    ) -> Result<()> {
        let ds = MockDataSource::new();
        ds.merge_write_set(main_ds.to_write_set()?);
        Self::store_meta_resources(step.meta(), &ds)?;

        let vm = Dvm::new(ds);

        let unit = byte_code_map
            .get(step.unit())
            .ok_or_else(|| anyhow!("Failed to resolve bytecode"))?
            .to_vec();

        let tx_meta = Self::make_execution_meta(step.meta())?;
        let result = match step {
            TestStep::PublishModule(_) => vm.publish_module(tx_meta, Module::new(unit)),
            TestStep::ExecuteScript(_) => {
                vm.execute_script(tx_meta, Script::new(unit, vec![], vec![]))
            }
        };

        Self::handle_tx_tesult(main_ds, &step.meta().expected_result, result)
    }

    /// Make vm execution meta.
    fn make_execution_meta(test_meta: &TestMeta) -> Result<ExecutionMeta> {
        ExecutionMeta::new(test_meta.gas, 1, test_meta.address)
    }

    /// Store mete resources.
    fn store_meta_resources(test_meta: &TestMeta, ds: &MockDataSource) -> Result<()> {
        for (ticker, price) in &test_meta.oracle_price_list {
            ds.insert(
                AccessPath::new(
                    CORE_CODE_ADDRESS,
                    oracle_metadata(&ticker.0, &ticker.1).access_vector(),
                ),
                lcs::to_bytes(&Price { price: *price })?,
            );
        }

        let block = BlockMetadata {
            height: test_meta.block,
        };
        ds.insert(
            AccessPath::new(CORE_CODE_ADDRESS, block_metadata().access_vector()),
            lcs::to_bytes(&block)?,
        );

        let timestamp = CurrentTimestamp {
            seconds: test_meta.time,
        };
        ds.insert(
            AccessPath::new(CORE_CODE_ADDRESS, time_metadata().access_vector()),
            lcs::to_bytes(&timestamp)?,
        );
        Ok(())
    }

    /// Handles transaction resources.
    fn handle_tx_tesult(
        ds: &MockDataSource,
        expected_result: &ExecutionResult,
        result: VmResult,
    ) -> Result<()> {
        match expected_result {
            ExecutionResult::Success => match result {
                Ok(result) => {
                    let major_status = result.status.major_status();
                    if major_status == StatusCode::EXECUTED {
                        ds.merge_write_set(result.write_set);
                        Ok(())
                    } else {
                        Err(anyhow!(
                            "Unexpected execution result [{:?}]. Success status is expected.",
                            result.status
                        ))
                    }
                }
                Err(err) => Err(anyhow!(
                    "Unexpected execution result [{:?}]. Success status is expected.",
                    err
                )),
            },
            ExecutionResult::Error {
                main_status,
                additional_status,
            } => {
                let status = match result {
                    Ok(result) => result.status.into_vm_status(),
                    Err(status) => status,
                };

                if status.status_code() == StatusCode::EXECUTED {
                    return Err(anyhow!(
                        "Unexpected execution result [{:?}]. Error status is expected.",
                        status
                    ));
                }

                if let Some(major_status) = main_status {
                    if status.status_code() as u64 != *major_status {
                        return Err(anyhow!("Unexpected execution result [{:?}]. {:?} major status status is expected.", status, major_status));
                    }
                }

                if let Some(additional_status) = additional_status {
                    if status.move_abort_code() != Some(*additional_status) {
                        return Err(anyhow!("Unexpected execution result [{:?}]. {:?} additional status status is expected.", status, additional_status));
                    }

                    if *main_status == None && status.status_code() != StatusCode::ABORTED {
                        return Err(anyhow!("Unexpected execution result [{:?}]. ABORTED major status status is expected.", status));
                    }
                }

                Ok(())
            }
        }
    }
}
