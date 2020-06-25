mod pipeline;
mod runner;

use lang::{
    stdlib::{build_std},
};
use std::collections::HashMap;
use crate::test_suite::runner::TestState;
use termion::color;

/// Runs move test suite.
pub fn run_test_suite(suite: HashMap<String, String>) {
    let stdlib = build_std();

    let mut has_error = false;
    for (test_name, content) in suite {
        let test = TestState::new(stdlib.clone(), test_name.clone(), content);
        match test.perform() {
            Ok(_) => {
                println!(
                    "{}Test: {} - {}OK",
                    color::Fg(color::Black),
                    test_name,
                    color::Fg(color::Green)
                );
            }
            Err(err) => {
                println!(
                    "{}Test: {} - {}Error",
                    color::Fg(color::Black),
                    test_name,
                    color::Fg(color::Red)
                );
                println!("{}", err);
                has_error = true;
            }
        }
    }
    if has_error {
        panic!();
    }
}
