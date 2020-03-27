#[macro_use]
extern crate anyhow;
extern crate libra;

pub mod banch32;
pub mod bytecode;
pub mod compiler;
pub mod stdlib;

#[macro_export]
macro_rules! pattern {
    ($re:literal $(,)?) => {{
        static RE: once_cell::sync::OnceCell<regex::Regex> = once_cell::sync::OnceCell::new();
        RE.get_or_init(|| regex::Regex::new($re).unwrap())
    }};
}
