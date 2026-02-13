#[cfg(feature = "r1")]
#[allow(clippy::all)]
#[allow(clippy::pedantic)]
#[allow(unsafe_code)]
mod gen_r1;
#[cfg(feature = "r3")]
#[allow(clippy::all)]
#[allow(clippy::pedantic)]
#[allow(unsafe_code)]
mod gen_r3;
#[cfg(feature = "r4")]
#[allow(clippy::all)]
#[allow(clippy::pedantic)]
#[allow(unsafe_code)]
mod gen_r4;
#[cfg(feature = "r5")]
#[allow(clippy::all)]
#[allow(clippy::pedantic)]
#[allow(unsafe_code)]
mod gen_r5;
#[cfg(feature = "r1")]
pub mod r1;
#[cfg(feature = "r3")]
pub mod r3;
#[cfg(feature = "r4")]
pub mod r4;
#[cfg(feature = "r5")]
pub mod r5;
