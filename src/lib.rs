//! This is a platform-agnostic Rust driver for the Texas Instruments BQ40Z50 Battery
//! fuel/gas gauge based on the [`embedded-hal`] traits.
//!
//! [`embedded-hal`]: https://docs.rs/embedded-hal
//!
//! For further details of the device architecture and operation, please refer
//! to one of the official Datasheets:
//!
//! - [`Datasheet R1`]
//! - [`Datasheet R3`]
//! - [`Datasheet R4`]
//! - [`Datasheet R5`]
//!
//! [`Datasheet R1`]: https://www.ti.com/lit/ug/sluua43a/sluua43a.pdf
//! [`Datasheet R3`]: https://www.ti.com/lit/ug/sluubu5a/sluubu5a.pdf
//! [`Datasheet R4`]: https://www.ti.com/lit/ug/sluuch2/sluuch2.pdf
//! [`Datasheet R5`]: https://www.ti.com/lit/ug/sluucn4b/sluucn4b.pdf

#![doc = include_str!("../README.md")]
#![cfg_attr(not(test), no_std)]
#![allow(missing_docs)]

mod common;
mod consts;
mod error;
mod interface;
mod tests;
mod versions;

// Re-export types for public use
#[cfg(feature = "r1")]
pub use versions::r1::Bq40z50R1;
#[cfg(feature = "r3")]
pub use versions::r3::Bq40z50R3;
#[cfg(feature = "r4")]
pub use versions::r4::Bq40z50R4;
#[cfg(feature = "r5")]
pub use versions::r5::Bq40z50R5;

pub use error::BQ40Z50Error;
