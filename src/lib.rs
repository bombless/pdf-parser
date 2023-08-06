#[macro_use]
extern crate lazy_static;

pub mod parser;
pub mod lexer;

pub use parser::{Value, Object, PDF};
