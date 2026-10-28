//! Datum Calc core: everything except the window. No UI or OS code lives here.
pub mod eval;
pub mod latex;
pub mod parse;
pub mod sheet;
pub mod solve;
pub mod units;

pub use sheet::{Doc, Kind, Op, Region, Sheet};

#[cfg(test)]
mod hardening;
#[cfg(test)]
mod tests;
