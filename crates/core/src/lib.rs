//! Datum Calc core: everything except the window. No UI or OS code lives here.
pub mod sheet;

pub use sheet::{Doc, Kind, Op, Region, Sheet};

#[cfg(test)]
mod hardening;
