//! Mini payment processor: registers clients, applies credits and debits, and runs cut-offs.
#![warn(missing_docs)]

pub mod application;
pub mod config;
pub mod domain;
pub mod http;
pub mod infrastructure;
