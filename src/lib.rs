//! This is a platform-agnostic Rust driver for the Texas Instruments BQ40Z50 Battery
//! fuel/gas gauge based on the [`embedded-hal`] traits.
//!
//! [`embedded-hal`]: https://docs.rs/embedded-hal
//!
//! For further details of the device architecture and operation, please refer
//! to the official [`Datasheet`].
//!
//! [`Datasheet`]: https://www.ti.com/lit/ug/sluua43a/sluua43a.pdf

// #![doc = include_str!("../README.md")]
// #![no_std]

#![doc = include_str!("../README.md")]
#![cfg_attr(not(test), no_std)]
#![allow(missing_docs)]
device_driver::create_device!(
    device_name: Device,
    manifest: "device.yaml"
);
