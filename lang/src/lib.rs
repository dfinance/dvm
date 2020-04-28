#[macro_use]
extern crate anyhow;
extern crate libra;
#[macro_use]
extern crate include_dir;

pub mod banch32;
pub mod bytecode;
pub mod compiler;
pub mod module_checker;
pub mod stdlib;

#[macro_export]
macro_rules! pattern {
    ($re:literal $(,)?) => {{
        static RE: once_cell::sync::OnceCell<regex::Regex> = once_cell::sync::OnceCell::new();
        RE.get_or_init(|| regex::Regex::new($re).unwrap())
    }};
}
