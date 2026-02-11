#![allow(missing_docs)]

use bq40z50_rx::Bq40z50R5;
#[tokio::main]
async fn main() {
    let hal = pico_de_gallo_hal::Hal::new();
    let i2c = hal.i2c();
    let delay = hal.delay();

    let mut bq = Bq40z50R5::new(i2c, delay);

    let c = bq.device.afe_reg().read_async().await.unwrap();
    let d = bq40z50_rx::r5::field_sets::AfeReg::new();
}
