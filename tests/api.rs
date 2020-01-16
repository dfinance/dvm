use move_vm_in_cosmos::test_kit::{TestKit, Lang};
use std::thread;
use tokio::runtime::Runtime;

#[test]
fn test() {
    let test_kit = TestKit::new(Lang::MvIr);
    thread::park_timeout_ms(10000);

}

#[test]
fn test_1() {
    let test_kit = TestKit::new(Lang::Move);
    thread::park_timeout_ms(10000);

}