//! Datum Calc core: everything except the window. No UI or OS code lives here.
pub mod docx;
pub mod dynsys;
pub mod eval;
pub mod fluids;
pub mod latex;
pub mod materials;
pub mod ode;
pub mod omml;
pub mod packs;
pub mod parse;
pub mod prog;
pub mod propfn;
pub mod sheet;
pub mod solve;
pub mod sym;
pub mod tabin;
pub mod units;

pub use sheet::{Doc, Kind, Op, Region, Sheet};

#[cfg(test)]
mod hardening;
#[cfg(test)]
mod tests;
