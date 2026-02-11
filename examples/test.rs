#![allow(missing_docs)]

use bq40z50_rx::Bq40z50R5;
use embedded_hal::spi::SpiBus;
fn main() {
    let hal = pico_de_gallo_hal::Hal::new();
    let i2c = hal.i2c();
    let delay = hal.delay();
    let gpio = hal.gpio(0);
    let mut spi = hal.spi();

    spi.write(&[9]);

    let bq = Bq40z50R5::new(i2c, delay);
}
