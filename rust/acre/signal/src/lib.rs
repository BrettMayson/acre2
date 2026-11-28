//! ACRE's radio signal propagation, ported from the C++ signal extension (`acre_x64.dll`).
//!
//! - [`wrp`] / [`pbo`] / [`open_files`]: find and read the terrain from the game's PBOs.
//! - [`map`]: the elevation grid and peaks.
//! - [`antenna`]: gain patterns.
//! - [`models`]: arcade, LOS simple, LOS multipath, Longley-Rice ITM.
//! - [`signal_map`]: the signal map debug tool.
//! - [`engine`]: all of it behind the commands SQF uses.
//!
//! The models keep the C++'s `f32` arithmetic (and its quirks, except where noted) so radios
//! behave as before; `tests/golden.rs` compares them with the original.

#![allow(clippy::cast_lossless)]

pub mod antenna;
pub mod compress;
pub mod engine;
pub mod map;
pub mod math;
pub mod models;
pub mod open_files;
pub mod pbo;
pub mod signal_map;
pub mod wrp;

pub use engine::{Engine, Model, Request};
pub use math::Vec3;
