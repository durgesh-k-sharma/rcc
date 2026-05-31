//! Core shared data structures for the rcc compiler.
//!
//! This crate provides types used across all other crates:
//! - Source file management and source locations
//! - Interned strings and identifiers
//! - Diagnostic infrastructure

pub mod source;
pub mod ident;
pub mod diagnostics;

pub use source::{FileId, SourceFile, SourceManager, Span};
pub use ident::{Ident, Interner};
pub use diagnostics::{Diagnostic, Severity, Diagnostics};
