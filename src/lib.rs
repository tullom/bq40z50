//! This is a platform-agnostic Rust driver for the Texas Instruments BQ40Z50 Battery
//! fuel/gas gauge based on the [`embedded-hal`] traits.
//!
//! [`embedded-hal`]: https://docs.rs/embedded-hal
//!
//! For further details of the device architecture and operation, please refer
//! to the official [`Datasheet`].
//!
//! [`Datasheet`]: https://www.ti.com/lit/ug/sluua43a/sluua43a.pdf

#![doc = include_str!("../README.md")]
#![cfg_attr(not(test), no_std)]
#![allow(missing_docs)]

use embedded_hal_async::i2c::I2c as I2cTrait;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
/// BQ40Z50 Errors
pub enum BQ40Z50Error<I2cError> {
    I2c(I2cError),
}

const BQ_ADDR: u8 = 0x0B;
const LARGEST_REG_SIZE_BYTES: usize = 5;
const LARGEST_CMD_SIZE_BYTES: usize = 32;
const LARGEST_BUF_SIZE_BYTES: usize = 33;
const MAC_CMD_ADDR_SIZE_BYTES: usize = 2;
const MAC_CMD_ADDR_SIZE_BITS: usize = MAC_CMD_ADDR_SIZE_BYTES * 8;

/// BQ40Z50 interface, which takes an async I2C bus
pub struct DeviceInterface<I2c: I2cTrait> {
    /// embedded-hal-async compliant I2C bus
    pub i2c: I2c,
}

device_driver::create_device!(
    device_name: Device,
    manifest: "device.yaml"
);

impl<I2c: I2cTrait> device_driver::AsyncRegisterInterface for DeviceInterface<I2c> {
    type Error = BQ40Z50Error<I2c::Error>;
    type AddressType = u8;

    async fn write_register(
        &mut self,
        address: Self::AddressType,
        _size_bits: u32,
        data: &[u8],
    ) -> Result<(), Self::Error> {
        debug_assert!((data.len() <= LARGEST_REG_SIZE_BYTES), "Register size too big");

        // Add one byte for register address
        let mut buf = [0u8; 1 + LARGEST_REG_SIZE_BYTES];
        buf[0] = address;
        buf[1..=data.len()].copy_from_slice(data);

        // Because the BQ40Z50's registers vary in size, we pass in a slice of
        // the appropriate size so we do not accidentally write to the register
        // at address + 1 when writing to a 1 byte register
        self.i2c
            .write(BQ_ADDR, &buf[..=data.len()])
            .await
            .map_err(BQ40Z50Error::I2c)
    }

    async fn read_register(
        &mut self,
        address: Self::AddressType,
        _size_bits: u32,
        data: &mut [u8],
    ) -> Result<(), Self::Error> {
        self.i2c
            .write_read(BQ_ADDR, &[address], data)
            .await
            .map_err(BQ40Z50Error::I2c)
    }
}

impl<I2c: I2cTrait> device_driver::AsyncCommandInterface for DeviceInterface<I2c> {
    type Error = BQ40Z50Error<I2c::Error>;
    type AddressType = u32;

    async fn dispatch_command(
        &mut self,
        address: Self::AddressType,
        _size_bits_in: u32,
        input: &[u8],
        _size_bits_out: u32,
        output: &mut [u8],
    ) -> Result<(), Self::Error> {
        debug_assert!((input.len() <= LARGEST_CMD_SIZE_BYTES), "Command size too big");

        let mut buf = [0u8; 1 + MAC_CMD_ADDR_SIZE_BYTES + LARGEST_CMD_SIZE_BYTES];
        buf[0] = ((address >> MAC_CMD_ADDR_SIZE_BITS) & 0xFF) as u8;
        buf[1] = ((address >> 8) & 0xFF) as u8;
        buf[2] = (address & 0xFF) as u8;
        buf[3..input.len() + 3].copy_from_slice(input);
        self.i2c
            .write_read(BQ_ADDR, &buf[..=input.len() + MAC_CMD_ADDR_SIZE_BYTES], output)
            .await
            .map_err(BQ40Z50Error::I2c)
    }
}

impl<I2c: I2cTrait> device_driver::BufferInterfaceError for DeviceInterface<I2c> {
    type Error = BQ40Z50Error<I2c::Error>;
}

impl<I2c: I2cTrait> device_driver::AsyncBufferInterface for DeviceInterface<I2c> {
    type AddressType = u8;

    async fn read(&mut self, address: Self::AddressType, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.i2c
            .write_read(BQ_ADDR, &[address], buf)
            .await
            .map_err(BQ40Z50Error::I2c)?;
        Ok(buf.len())
    }

    async fn write(&mut self, address: Self::AddressType, buf: &[u8]) -> Result<usize, Self::Error> {
        debug_assert!((buf.len() <= LARGEST_BUF_SIZE_BYTES), "Buffer size too big");

        // Add one byte for register address
        let mut data = [0u8; 1 + LARGEST_BUF_SIZE_BYTES];
        data[0] = address;
        data[1..=buf.len()].copy_from_slice(buf);
        self.i2c.write(BQ_ADDR, &data).await.map_err(BQ40Z50Error::I2c)?;
        Ok(buf.len())
    }

    async fn flush(&mut self, _address: Self::AddressType) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use embedded_hal_mock::eh1::i2c::{Mock, Transaction};

    use super::*;

    #[tokio::test]
    async fn read_chip_id() {
        let expectations = vec![Transaction::write_read(BQ_ADDR, vec![0x44, 0x21, 0x00], vec![])];
        let i2c = Mock::new(&expectations);
        let mut bq = Device::new(DeviceInterface { i2c });

        bq.mac_gauging().dispatch_async().await.unwrap();

        bq.interface.i2c.done();
    }

    #[tokio::test]
    async fn read_chip_id_2() {
        let expectations = vec![Transaction::write_read(
            BQ_ADDR,
            vec![0x44, 0x01, 0x00],
            vec![0x00, 0x00],
        )];
        let i2c = Mock::new(&expectations);
        let mut bq = Device::new(DeviceInterface { i2c });

        bq.mac_device_type().dispatch_async().await.unwrap();
        bq.interface.i2c.done();
    }

    #[tokio::test]
    async fn read_too_large_manufacture_name() {
        let expectations = vec![Transaction::write_read(BQ_ADDR, vec![0x20], vec![0x00])];
        let i2c = Mock::new(&expectations);
        let mut bq = Device::new(DeviceInterface { i2c });

        let mut manufacture_name = [0u8; 1];

        bq.manufacture_name().read_async(&mut manufacture_name).await.unwrap();
        bq.interface.i2c.done();
    }

    #[tokio::test]
    async fn write_unseal_keys() {
        let expectations = vec![Transaction::write_read(
            BQ_ADDR,
            vec![0x44, 0x35, 0x00, 0x30, 0x30, 0x60, 0x60, 0x01, 0x01, 0x10, 0x10],
            vec![0x30, 0x30, 0x60, 0x60, 0x01, 0x01, 0x10, 0x10],
        )];
        let i2c = Mock::new(&expectations);
        let mut bq = Device::new(DeviceInterface { i2c });

        let f = bq
            .mac_security_keys()
            .dispatch_async(|f| {
                f.set_unseal_key_a(0x3030);
                f.set_unseal_key_b(0x6060);
                f.set_full_access_key_a(0x0101);
                f.set_full_access_key_b(0x1010);
            })
            .await
            .unwrap();

        assert_eq!(f.unseal_key_a(), 0x3030);
        assert_eq!(f.unseal_key_b(), 0x6060);
        assert_eq!(f.full_access_key_a(), 0x0101);
        assert_eq!(f.full_access_key_b(), 0x1010);
        bq.interface.i2c.done();
    }
}
