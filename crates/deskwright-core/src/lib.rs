//! Domain core of Deskwright: models, port traits implemented by platform adapters,
//! and the use-case services that orchestrate them. Contains no OS or protocol code.

pub mod element_registry;
pub mod error;
pub mod model;
pub mod polling;
pub mod ports;
pub mod services;

#[cfg(any(test, feature = "test-support"))]
pub mod testing;

pub use error::{DeskwrightError, DeskwrightResult, Permission};
