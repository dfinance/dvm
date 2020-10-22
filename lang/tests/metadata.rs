use libra::prelude::{CORE_CODE_ADDRESS, SignatureToken};
use libra::file_format::Kind;
use compiler::Compiler;
use ds::MockDataSource;
use dvm_lang::bytecode::metadata::*;
use stdlib::build_std;

pub fn compile(source: &str) -> Vec<u8> {
    let ds = MockDataSource::with_write_set(build_std());
    let compiler = Compiler::new(ds);
    compiler.compile(source, Some(CORE_CODE_ADDRESS)).unwrap()
}

#[test]
fn test_script_metadata() {
    let script = compile(
        r"
            script {
                fun main() {}
            }
        ",
    );
    let meta = extract_bytecode_metadata(&script).unwrap();
    assert_eq!(
        meta,
        Metadata::Script {
            type_parameters: vec![],
            arguments: vec![],
        }
    );

    let script = compile(
        r"
            script {
                fun main(_signer: &signer,
                         _signer_2: &signer,
                         _signer_3: &signer,
                         _1: u8,
                         _2: u64,
                         _3: u128,
                         _4: bool,
                         _5: address,
                         _6: vector<u8>) {}
            }
        ",
    );
    let meta = extract_bytecode_metadata(&script).unwrap();
    assert_eq!(
        meta,
        Metadata::Script {
            type_parameters: vec![],
            arguments: vec![
                SignatureToken::Reference(Box::new(SignatureToken::Signer)),
                SignatureToken::Reference(Box::new(SignatureToken::Signer)),
                SignatureToken::Reference(Box::new(SignatureToken::Signer)),
                SignatureToken::U8,
                SignatureToken::U64,
                SignatureToken::U128,
                SignatureToken::Bool,
                SignatureToken::Address,
                SignatureToken::Vector(Box::new(SignatureToken::U8)),
            ],
        }
    );

    let script = compile(
        r"
            script {
                fun main<T, R: resource, G: copyable>(_signer: &signer, _6: vector<u8>) {}
            }
        ",
    );
    let meta = extract_bytecode_metadata(&script).unwrap();
    assert_eq!(
        meta,
        Metadata::Script {
            type_parameters: vec![Kind::All, Kind::Resource, Kind::Copyable],
            arguments: vec![
                SignatureToken::Reference(Box::new(SignatureToken::Signer)),
                SignatureToken::Vector(Box::new(SignatureToken::U8)),
            ],
        }
    );
}

#[test]
fn test_module_metadata() {
    let module = compile(
        r"
            address 0x01 {
                module Foo {
                    use 0x01::Block;

                    struct Type1<T, D> {
                        f1: u8,
                        f2: u64,
                        f3: u128,
                        f4: bool,
                        f5: address,
                        f6: vector<T>,
                        f7: D
                    }

                    struct Type2 {}

                    resource struct Type3<R> {
                        foo: R
                    }

                    struct Type4 {
                        f1: Type2,
                    }

                    resource struct Type5 {
                        f1: Block::BlockMetadata,
                        f2: 0x01::Coins::ETH
                    }

                    native public fun f1();
                    native fun f2(a: u64): u64;
                    native fun f3<T>(r: &T): &T;
                    native fun f4<T>(c: &mut Type3<T>): &mut Type3<T>;
                    native fun f5<T>(t: 0x01::Coins::ETH): (Block::BlockMetadata, u8);
                    public fun f6() {}
                }
            }
        ",
    );
    let meta = extract_bytecode_metadata(&module).unwrap();

    fn own(s: &str) -> String {
        s.to_owned()
    }

    fn field(name: &str, f_type: &str) -> FieldMeta {
        FieldMeta {
            name: own(name),
            f_type: own(f_type),
        }
    }

    fn fun(
        name: &str,
        is_public: bool,
        is_native: bool,
        type_params: &[&str],
        arguments: &[&str],
        ret: &[&str],
    ) -> FunctionMeta {
        FunctionMeta {
            name: own(name),
            is_public,
            is_native,
            type_params: type_params.iter().map(ToString::to_string).collect(),
            arguments: arguments.iter().map(ToString::to_string).collect(),
            ret: ret.iter().map(ToString::to_string).collect(),
        }
    }

    assert_eq!(
        meta,
        Metadata::Module {
            name: own("Foo"),
            functions: vec![
                fun("f1", true, true, &[], &[], &[]),
                fun("f2", false, true, &[], &["u64"], &["u64"]),
                fun("f3", false, true, &["T"], &["&T"], &["&T"]),
                fun(
                    "f4",
                    false,
                    true,
                    &["T"],
                    &["&mut Type3<T>"],
                    &["&mut Type3<T>"]
                ),
                fun(
                    "f5",
                    false,
                    true,
                    &["T"],
                    &["0x01::Coins::ETH"],
                    &["0x01::Block::BlockMetadata", "u8"]
                ),
                fun("f6", true, false, &[], &[], &[]),
            ],
            structs: vec![
                StructMeta {
                    name: own("Type1"),
                    is_resource: false,
                    type_params: vec![own("T"), own("T1")],
                    fields: vec![
                        field("f1", "u8"),
                        field("f2", "u64"),
                        field("f3", "u128"),
                        field("f4", "bool"),
                        field("f5", "address"),
                        field("f6", "vector<T>"),
                        field("f7", "T1")
                    ],
                },
                StructMeta {
                    name: own("Type2"),
                    is_resource: false,
                    type_params: vec![],
                    fields: vec![],
                },
                StructMeta {
                    name: own("Type3"),
                    is_resource: true,
                    type_params: vec![own("T")],
                    fields: vec![field("foo", "T")],
                },
                StructMeta {
                    name: own("Type4"),
                    is_resource: false,
                    type_params: vec![],
                    fields: vec![field("f1", "Type2")],
                },
                StructMeta {
                    name: own("Type5"),
                    is_resource: true,
                    type_params: vec![],
                    fields: vec![
                        field("f1", "0x01::Block::BlockMetadata"),
                        field("f2", "0x01::Coins::ETH")
                    ],
                }
            ],
        }
    );
}
