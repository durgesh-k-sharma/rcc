//! rcc-backend: Machine IR, instruction selection, register allocation,
//! ABI implementation, and x86-64 assembly emission.

#![allow(dead_code)]

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
