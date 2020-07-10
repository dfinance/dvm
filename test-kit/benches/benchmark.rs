use criterion::{criterion_group, criterion_main, Criterion};
use lang::{stdlib::build_std};
use data_source::MockDataSource;
use compiler::{Compiler, disassembler};
use libra::prelude::*;

/// Prepare compilation benchmark setup;
fn compiler_setup(ds: &MockDataSource) -> (Compiler<MockDataSource>, &'static str) {
    let compiler = Compiler::new(ds.clone());
    (compiler, include_str!("assets/bench_sample.move"))
}

/// Prepare compilation benchmark setup;
fn prepare_disassembler_setup(compiler: &Compiler<MockDataSource>, source: &str) -> Vec<u8> {
    compiler.compile(source, Some(CORE_CODE_ADDRESS)).unwrap()
}

/// Perform disassembler benchmark.
fn disassemble(bytecode: Vec<u8>) {
    disassembler::module_signature(&bytecode)
        .unwrap()
        .to_string();
}

/// Performs benchmarks.
fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("build_stdlib", |b| b.iter(build_std));

    let ds = MockDataSource::with_write_set(build_std());
    c.bench_function("compiled_module", |b| {
        b.iter_with_large_setup(
            || compiler_setup(&ds),
            |(compiler, source)| {
                compiler
                    .compile(source, Some(AccountAddress::random()))
                    .unwrap()
            },
        )
    });

    let compiler = Compiler::new(ds.clone());
    ds.publish_module(
        compiler
            .compile(
                include_str!("../../compiler/tests/resources/disassembler/base.move"),
                Some(AccountAddress::new([0x1; 20])),
            )
            .unwrap(),
    )
    .unwrap();
    ds.publish_module(
        compiler
            .compile(
                include_str!("../../compiler/tests/resources/disassembler/base_1.move"),
                Some(CORE_CODE_ADDRESS),
            )
            .unwrap(),
    )
    .unwrap();

    c.bench_function("disassemble_empty_module", |b| {
        b.iter_with_large_setup(
            || {
                prepare_disassembler_setup(
                    &compiler,
                    include_str!("../../compiler/tests/resources/disassembler/empty_module.move"),
                )
            },
             disassemble,
        )
    });
    c.bench_function("disassemble_functions", |b| {
        b.iter_with_large_setup(
            || {
                prepare_disassembler_setup(
                    &compiler,
                    include_str!(
                        "../../compiler/tests/resources/disassembler/module_with_functions.move"
                    ),
                )
            },
            disassemble,
        )
    });
    c.bench_function("disassemble_structs", |b| {
        b.iter_with_large_setup(
            || {
                prepare_disassembler_setup(
                    &compiler,
                    include_str!(
                        "../../compiler/tests/resources/disassembler/module_with_structs.move"
                    ),
                )
            },
            disassemble,
        )
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
