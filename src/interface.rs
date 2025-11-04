use core::hash::Hasher;
use embedded_hal_async::delay::DelayNs as DelayTrait;
use embedded_hal_async::i2c::I2c as I2cTrait;

use crate::consts::*;
use crate::{common::Config, error::BQ40Z50Error};

#[cfg(feature = "embassy-timeout")]
use embassy_time::with_timeout;

/// BQ40Z50 interface, common to all chip revisions, which takes an async I2C bus
pub struct DeviceInterface<I2C: I2cTrait, DELAY: DelayTrait> {
    /// embedded-hal-async compliant I2C bus
    pub i2c: I2C,
    pub delay: DELAY,
    pub config: Config,
}

impl<I2C: I2cTrait, DELAY: DelayTrait> DeviceInterface<I2C, DELAY> {
    pub fn new(i2c: I2C, delay: DELAY) -> Self {
        DeviceInterface {
            i2c,
            delay,
            config: Config::default(),
        }
    }
}

impl<I2C: I2cTrait, DELAY: DelayTrait> DeviceInterface<I2C, DELAY> {
    pub(crate) async fn mac_write_with_retries(&mut self, write: &[u8]) -> Result<(), BQ40Z50Error<I2C::Error>> {
        // Same functionality as regular SMBus writes, write buffer just needs to be properly formed.
        self.write_with_retries(write).await
    }

    pub(crate) async fn mac_write_with_retries_pec(&mut self, write: &[u8]) -> Result<(), BQ40Z50Error<I2C::Error>> {
        // Same functionality as regular SMBus writes, write buffer just needs to be properly formed.
        self.write_with_retries_pec(write).await
    }

    #[allow(clippy::cast_possible_truncation)]
    pub(crate) async fn mac_write_to_df_with_retries(
        &mut self,
        starting_address: u16,
        write: &[u8],
    ) -> Result<(), BQ40Z50Error<I2C::Error>> {
        let mut bytes_left_to_write = write.len();
        while bytes_left_to_write > 0 {
            // Largest single write block is 1 byte MAC command + 1 byte size + 2 bytes starting address + 32 bytes data.
            let mut output_buf = [0u8; 4 + LARGEST_DF_BLOCK_SIZE_BYTES];
            // Determine how many bytes to write to the bus for this chunk.
            let output_buf_end_idx = core::cmp::min(output_buf.len(), bytes_left_to_write + 4);

            let start_idx = write.len() - bytes_left_to_write;
            let end_idx = start_idx + output_buf_end_idx - 4;
            // Safe cast as start_idx being higher than u16::MAX is impossible, the register map doesn't even go that high.
            let starting_address_chunk = (starting_address + start_idx as u16).to_le_bytes();
            output_buf[0] = MAC_CMD;
            // Safe cast as output_buf_end_idx can only be as high as output_buf.len(), which is 36
            output_buf[1] = output_buf_end_idx as u8 - 2;
            output_buf[2] = starting_address_chunk[0];
            output_buf[3] = starting_address_chunk[1];
            output_buf[4..output_buf_end_idx].copy_from_slice(&write[start_idx..end_idx]);

            self.write_with_retries(&output_buf[..output_buf_end_idx]).await?;
            bytes_left_to_write = bytes_left_to_write.saturating_sub(LARGEST_DF_BLOCK_SIZE_BYTES);
        }

        Ok(())
    }

    #[allow(clippy::cast_possible_truncation)]
    pub(crate) async fn mac_write_to_df_with_retries_pec(
        &mut self,
        starting_address: u16,
        write: &[u8],
    ) -> Result<(), BQ40Z50Error<I2C::Error>> {
        let mut bytes_left_to_write = write.len();
        while bytes_left_to_write > 0 {
            // Largest single write block is 1 byte MAC command + 1 byte size + 2 bytes starting address + 32 bytes data + 1 PEC byte.
            let mut output_buf = [0u8; 4 + LARGEST_DF_BLOCK_SIZE_BYTES + 1];
            // Determine how many bytes to write to the bus for this chunk.
            let output_buf_end_idx = core::cmp::min(output_buf.len() - 1, bytes_left_to_write + 4);

            let start_idx = write.len() - bytes_left_to_write;
            let end_idx = start_idx + output_buf_end_idx - 4;
            // Safe cast as start_idx being higher than u16::MAX is impossible, the register map doesn't even go that high.
            let starting_address_chunk = (starting_address + start_idx as u16).to_le_bytes();
            output_buf[0] = MAC_CMD;
            // Safe cast as output_buf_end_idx can only be as high as output_buf.len(), which is 36
            output_buf[1] = output_buf_end_idx as u8 - 2;
            output_buf[2] = starting_address_chunk[0];
            output_buf[3] = starting_address_chunk[1];
            output_buf[4..output_buf_end_idx].copy_from_slice(&write[start_idx..end_idx]);
            // Add PEC at the end.
            let mut pec = smbus_pec::Pec::new();
            pec.write(&[BQ_ADDR << 1]);
            pec.write(&output_buf[..output_buf_end_idx]);
            // Safe cast as SMBUS PEC is a u8, returned value is u64 because of the Hasher trait.
            output_buf[output_buf_end_idx] = pec.finish() as u8;

            self.write_with_retries(&output_buf[..=output_buf_end_idx]).await?;
            bytes_left_to_write = bytes_left_to_write.saturating_sub(LARGEST_DF_BLOCK_SIZE_BYTES);
        }

        Ok(())
    }
}

#[cfg(not(feature = "embassy-timeout"))]
impl<I2C: I2cTrait, DELAY: DelayTrait> DeviceInterface<I2C, DELAY> {
    async fn write_with_retries_internal(&mut self, write: &[u8]) -> Result<(), BQ40Z50Error<I2C::Error>> {
        let mut retries = self.config.max_bus_retries;

        // Because the BQ40Z50's registers vary in size, we pass in a slice of
        // the appropriate size so we do not accidentally write to the register
        // at address + 1 when writing to a 1 byte register
        while let Err(e) = self.i2c.write(BQ_ADDR, write).await {
            if retries == 0 {
                return Err(BQ40Z50Error::I2c(e));
            }
            retries -= 1;
            // Delay 10ms since the fuel gauge might be "thinking" from a previous command
            self.delay.delay_ms(DEFAULT_ERROR_BACKOFF_DELAY_MS).await;
        }

        Ok(())
    }

    pub(crate) async fn write_with_retries(&mut self, write: &[u8]) -> Result<(), BQ40Z50Error<I2C::Error>> {
        self.write_with_retries_internal(write).await
    }

    pub(crate) async fn write_with_retries_pec(&mut self, write: &[u8]) -> Result<(), BQ40Z50Error<I2C::Error>> {
        // Buffer to hold the entire message to compute PEC on
        let mut pec_buf = [0u8; LARGEST_REG_SIZE_BYTES * 2];
        // Device Addr + Write Bit (0)
        pec_buf[0] = BQ_ADDR << 1;
        pec_buf[1..=write.len()].copy_from_slice(write);

        // Write one more byte (PEC)
        let mut write_buf = [0u8; 1 + LARGEST_REG_SIZE_BYTES];
        write_buf[..write.len()].copy_from_slice(write);
        write_buf[write.len()] = smbus_pec::pec(&pec_buf[..=write.len()]);

        // Include everything we want to write plus the PEC byte
        let write_buf = &write_buf[..=write.len()];

        self.write_with_retries_internal(write_buf).await
    }

    pub(crate) async fn read_with_retries(
        &mut self,
        write: &[u8],
        read: &mut [u8],
    ) -> Result<(), BQ40Z50Error<I2C::Error>> {
        let mut retries = self.config.max_bus_retries;

        while let Err(e) = self.i2c.write_read(BQ_ADDR, write, read).await {
            if retries == 0 {
                return Err(BQ40Z50Error::I2c(e));
            }
            retries -= 1;
            // Delay 10ms since the fuel gauge might be "thinking" from a previous command
            self.delay.delay_ms(DEFAULT_ERROR_BACKOFF_DELAY_MS).await;
        }

        Ok(())
    }

    pub(crate) async fn read_with_retries_pec(
        &mut self,
        write: &[u8],
        read: &mut [u8],
    ) -> Result<(), BQ40Z50Error<I2C::Error>> {
        let mut retries = self.config.max_bus_retries;

        // Buffer to hold the entire message, including write and read, to compute PEC on
        let mut pec_buf = [0u8; LARGEST_REG_SIZE_BYTES * 2];
        // Device Addr + Write Bit (0)
        pec_buf[0] = BQ_ADDR << 1;
        pec_buf[1..=write.len()].copy_from_slice(write);
        // Device Addr + Read Bit (1)
        pec_buf[write.len() + 1] = BQ_ADDR << 1 | 0x01;

        // Read one more byte (PEC)
        let mut read_buf = [0u8; 1 + LARGEST_REG_SIZE_BYTES];
        let read_buf = &mut read_buf[..=read.len()];

        loop {
            let res = self.i2c.write_read(BQ_ADDR, write, read_buf).await;

            if let Err(e) = res {
                if retries == 0 {
                    return Err(BQ40Z50Error::I2c(e));
                }
                retries -= 1;
                // Delay 10ms since the fuel gauge might be "thinking" from a previous command
                self.delay.delay_ms(DEFAULT_ERROR_BACKOFF_DELAY_MS).await;
                continue;
            }

            let recvd_pec = read_buf[read.len()];
            // Copy just read bytes to pec_buf, without the PEC byte
            pec_buf[2 + write.len()..2 + write.len() + read.len()].copy_from_slice(&read_buf[..read.len()]);

            // Check PEC
            if recvd_pec != smbus_pec::pec(&pec_buf[..2 + write.len() + read.len()]) {
                if retries == 0 {
                    return Err(BQ40Z50Error::Pec);
                }
                retries -= 1;
                // Delay 10ms since the fuel gauge might be "thinking" from a previous command
                self.delay.delay_ms(DEFAULT_ERROR_BACKOFF_DELAY_MS).await;
                continue;
            }

            // If all is good, copy bytes we read into read.
            read.copy_from_slice(&read_buf[..read.len()]);
            return Ok(());
        }
    }

    pub(crate) async fn mac_read_with_retries(
        &mut self,
        write: &[u8],
        read: &mut [u8],
    ) -> Result<(), BQ40Z50Error<I2C::Error>> {
        let mut retries = self.config.max_bus_retries;

        // Loop until no bus errors or max bus retries are hit.
        loop {
            // Block write intended register.
            let res = self.i2c.write(BQ_ADDR, write).await;

            if let Err(e) = res {
                if retries == 0 {
                    return Err(BQ40Z50Error::I2c(e));
                }
                self.delay.delay_ms(DEFAULT_ERROR_BACKOFF_DELAY_MS).await;
                retries -= 1;
                continue;
            }

            // For read only commands.
            // Block read using I2C write_read, sending 0x44 as the command.
            // Response looks like [ Length (1 byte) | Command (2 bytes) | Data (output.len() bytes)]
            let mut output_buf = [0u8; 1 + LARGEST_CMD_SIZE_BYTES + MAC_CMD_ADDR_SIZE_BYTES as usize];

            let res = self
                .i2c
                .write_read(
                    BQ_ADDR,
                    &[write[0]],
                    &mut output_buf[..1 + MAC_CMD_ADDR_SIZE_BYTES as usize + read.len()],
                )
                .await;

            if let Err(e) = res {
                if retries == 0 {
                    return Err(BQ40Z50Error::I2c(e));
                }
                self.delay.delay_ms(DEFAULT_ERROR_BACKOFF_DELAY_MS).await;
                retries -= 1;
                continue;
            }

            read.copy_from_slice(
                &output_buf
                    [(1 + MAC_CMD_ADDR_SIZE_BYTES as usize)..(1 + MAC_CMD_ADDR_SIZE_BYTES as usize + read.len())],
            );

            return Ok(());
        }
    }

    #[allow(clippy::range_plus_one)]
    pub(crate) async fn mac_read_with_retries_pec(
        &mut self,
        write: &[u8],
        read: &mut [u8],
    ) -> Result<(), BQ40Z50Error<I2C::Error>> {
        let mut retries = self.config.max_bus_retries;

        // Buffer to hold the entire message to compute PEC on
        let mut pec_buf = [0u8; 2 + LARGEST_CMD_SIZE_BYTES + 2 + MAC_CMD_ADDR_SIZE_BYTES as usize];
        pec_buf[0] = BQ_ADDR << 1;
        pec_buf[1..=write.len()].copy_from_slice(write);

        // Compute PEC for the Write Block
        // Write one more byte (PEC)
        // [ MAC_CMD (0x44) | CMD_SIZE | CMD_LSB | CMD_MSB | PEC ]
        let mut write_buf = [0u8; 1 + 2 + MAC_CMD_ADDR_SIZE_BYTES as usize];
        write_buf[..write.len()].copy_from_slice(write);
        write_buf[write.len()] = smbus_pec::pec(&pec_buf[..=write.len()]);

        // Include everything we want to write plus the PEC byte
        let write_buf = &write_buf[..=write.len()];

        // Read one more byte (PEC)
        let mut read_buf = [0u8; 1 + MAC_CMD_ADDR_SIZE_BYTES as usize + LARGEST_CMD_SIZE_BYTES];
        let read_buf = &mut read_buf[..1 + MAC_CMD_ADDR_SIZE_BYTES as usize + read.len() + 1];

        // Reuse pec_buf for the block read now that we've computed PEC for the block write.
        pec_buf[0] = BQ_ADDR << 1;
        pec_buf[1] = write_buf[0];
        pec_buf[2] = BQ_ADDR << 1 | 0x01;

        // Loop until no bus errors or max bus retries are hit.
        loop {
            // Block write intended register.
            let res = self.i2c.write(BQ_ADDR, write_buf).await;

            if let Err(e) = res {
                if retries == 0 {
                    return Err(BQ40Z50Error::I2c(e));
                }
                self.delay.delay_ms(DEFAULT_ERROR_BACKOFF_DELAY_MS).await;
                retries -= 1;
                continue;
            }

            // For read only commands.
            // Block read using I2C write_read, sending 0x44 as the command.
            // Response looks like [ Length (1 byte) | Command (2 bytes) | Data (output.len() bytes)]

            let res = self.i2c.write_read(BQ_ADDR, &[write_buf[0]], read_buf).await;

            if let Err(e) = res {
                if retries == 0 {
                    return Err(BQ40Z50Error::I2c(e));
                }
                self.delay.delay_ms(DEFAULT_ERROR_BACKOFF_DELAY_MS).await;
                retries -= 1;
                continue;
            }

            let recvd_pec = read_buf[1 + MAC_CMD_ADDR_SIZE_BYTES as usize + read.len()];
            // Copy just read bytes to pec_buf, without the PEC byte
            pec_buf[3..3 + 1 + MAC_CMD_ADDR_SIZE_BYTES as usize + read.len()]
                .copy_from_slice(&read_buf[..1 + MAC_CMD_ADDR_SIZE_BYTES as usize + read.len()]);

            // Check PEC
            if recvd_pec != smbus_pec::pec(&pec_buf[..3 + 1 + MAC_CMD_ADDR_SIZE_BYTES as usize + read.len()]) {
                if retries == 0 {
                    return Err(BQ40Z50Error::Pec);
                }
                retries -= 1;
                // Delay 10ms since the fuel gauge might be "thinking" from a previous command
                self.delay.delay_ms(DEFAULT_ERROR_BACKOFF_DELAY_MS).await;
                continue;
            }

            read.copy_from_slice(
                &read_buf[(1 + MAC_CMD_ADDR_SIZE_BYTES as usize)..(1 + MAC_CMD_ADDR_SIZE_BYTES as usize + read.len())],
            );

            return Ok(());
        }
    }

    pub(crate) async fn mac_read_from_df_with_retries(
        &mut self,
        starting_address: u16,
        read: &mut [u8],
    ) -> Result<(), BQ40Z50Error<I2C::Error>> {
        let mut retries = self.config.max_bus_retries;
        let starting_address = starting_address.to_le_bytes();

        // Loop until no bus errors or max bus retries are hit.
        loop {
            // Block write intended register.
            let res = self
                .i2c
                .write(
                    BQ_ADDR,
                    &[
                        MAC_CMD,
                        MAC_CMD_ADDR_SIZE_BYTES,
                        starting_address[0],
                        starting_address[1],
                    ],
                )
                .await;

            if let Err(e) = res {
                if retries == 0 {
                    return Err(BQ40Z50Error::I2c(e));
                }
                self.delay.delay_ms(DEFAULT_ERROR_BACKOFF_DELAY_MS).await;
                retries -= 1;
                continue;
            }

            // Read in 32 byte chunks. The FG supports an auto-increment on the address during a DF read.
            // If an SMBus read block is sent, the gauge will return 32 bytes of DF data,
            // and if a subsequent SMBus read block is sent with command 0x44,
            // the gauge returns another 32 bytes of DF data starting at the starting address + 32.
            let mut bytes_left_to_read = read.len();
            while bytes_left_to_read > 0 {
                // Largest single read block is 2 bytes starting address + 32 bytes data.
                let mut output_buf = [0u8; LARGEST_DF_BLOCK_SIZE_BYTES + MAC_CMD_ADDR_SIZE_BYTES as usize];
                // Determine how many bytes to read from the bus, ideally we want to minimize time reading from DF
                // so if we can read less than 32 bytes of DF data, do it.
                let output_buf_end_idx =
                    core::cmp::min(output_buf.len(), bytes_left_to_read + MAC_CMD_ADDR_SIZE_BYTES as usize);

                let res = self
                    .i2c
                    .write_read(BQ_ADDR, &[MAC_CMD], &mut output_buf[..output_buf_end_idx])
                    .await;

                if let Err(e) = res {
                    if retries == 0 {
                        return Err(BQ40Z50Error::I2c(e));
                    }
                    self.delay.delay_ms(DEFAULT_ERROR_BACKOFF_DELAY_MS).await;
                    retries -= 1;
                    continue;
                }

                let start_idx = read.len() - bytes_left_to_read;
                let end_idx = start_idx + output_buf_end_idx - MAC_CMD_ADDR_SIZE_BYTES as usize;
                read[start_idx..end_idx]
                    .copy_from_slice(&output_buf[(MAC_CMD_ADDR_SIZE_BYTES as usize)..output_buf_end_idx]);
                bytes_left_to_read = bytes_left_to_read.saturating_sub(LARGEST_DF_BLOCK_SIZE_BYTES);
            }

            return Ok(());
        }
    }

    pub(crate) async fn mac_read_from_df_with_retries_pec(
        &mut self,
        starting_address: u16,
        read: &mut [u8],
    ) -> Result<(), BQ40Z50Error<I2C::Error>> {
        let mut retries = self.config.max_bus_retries;
        let starting_address = starting_address.to_le_bytes();

        let pec = smbus_pec::pec(&[
            BQ_ADDR << 1,
            MAC_CMD,
            MAC_CMD_ADDR_SIZE_BYTES,
            starting_address[0],
            starting_address[1],
        ]);

        // Loop until no bus errors or max bus retries are hit.
        loop {
            // Block write intended register.
            let res = self
                .i2c
                .write(
                    BQ_ADDR,
                    &[
                        MAC_CMD,
                        MAC_CMD_ADDR_SIZE_BYTES,
                        starting_address[0],
                        starting_address[1],
                        pec,
                    ],
                )
                .await;

            if let Err(e) = res {
                if retries == 0 {
                    return Err(BQ40Z50Error::I2c(e));
                }
                self.delay.delay_ms(DEFAULT_ERROR_BACKOFF_DELAY_MS).await;
                retries -= 1;
                continue;
            }

            // Read in 32 byte chunks. The FG supports an auto-increment on the address during a DF read.
            // If an SMBus read block is sent, the gauge will return 32 bytes of DF data,
            // and if a subsequent SMBus read block is sent with command 0x44,
            // the gauge returns another 32 bytes of DF data starting at the starting address + 32.
            let mut bytes_left_to_read = read.len();
            while bytes_left_to_read > 0 {
                // Largest single read block is 2 bytes starting address + 32 bytes data + 1 PEC byte.

                let mut output_buf = [0u8; LARGEST_DF_BLOCK_SIZE_BYTES + MAC_CMD_ADDR_SIZE_BYTES as usize + 1];
                // Determine how many bytes to read from the bus, ideally we want to minimize time reading from DF
                // so if we can read less than 32 bytes of DF data, do it.
                let output_buf_end_idx = core::cmp::min(
                    output_buf.len(),
                    bytes_left_to_read + MAC_CMD_ADDR_SIZE_BYTES as usize + 1,
                );

                let res = self
                    .i2c
                    .write_read(BQ_ADDR, &[MAC_CMD], &mut output_buf[..output_buf_end_idx])
                    .await;

                if let Err(e) = res {
                    if retries == 0 {
                        return Err(BQ40Z50Error::I2c(e));
                    }
                    self.delay.delay_ms(DEFAULT_ERROR_BACKOFF_DELAY_MS).await;
                    retries -= 1;
                    continue;
                }

                let recvd_pec = output_buf[output_buf_end_idx - 1];
                let mut pec = smbus_pec::Pec::new();
                pec.write(&[BQ_ADDR << 1, MAC_CMD, BQ_ADDR << 1 | 0x01]);
                // Omit PEC
                pec.write(&output_buf[..output_buf_end_idx - 1]);
                let pec = pec.finish();

                if u64::from(recvd_pec) != pec {
                    if retries == 0 {
                        return Err(BQ40Z50Error::Pec);
                    }
                    self.delay.delay_ms(DEFAULT_ERROR_BACKOFF_DELAY_MS).await;
                    retries -= 1;
                    continue;
                }

                let start_idx = read.len() - bytes_left_to_read;
                let end_idx = start_idx + output_buf_end_idx - MAC_CMD_ADDR_SIZE_BYTES as usize - 1;
                read[start_idx..end_idx]
                    .copy_from_slice(&output_buf[(MAC_CMD_ADDR_SIZE_BYTES as usize)..output_buf_end_idx - 1]);
                bytes_left_to_read = bytes_left_to_read.saturating_sub(LARGEST_DF_BLOCK_SIZE_BYTES);
            }

            return Ok(());
        }
    }
}

#[cfg(feature = "embassy-timeout")]
impl<I2C: I2cTrait, DELAY: DelayTrait> DeviceInterface<I2C, DELAY> {
    async fn write_with_retries_internal(&mut self, write: &[u8]) -> Result<(), BQ40Z50Error<I2C::Error>> {
        let mut retries = self.config.max_bus_retries;

        // Because the BQ40Z50's registers vary in size, we pass in a slice of
        // the appropriate size so we do not accidentally write to the register
        // at address + 1 when writing to a 1 byte register
        loop {
            let res = match with_timeout(self.config.timeout, self.i2c.write(BQ_ADDR, write)).await {
                Err(_) => Err(BQ40Z50Error::Timeout),
                Ok(Err(bus_err)) => Err(BQ40Z50Error::I2c(bus_err)),
                Ok(Ok(())) => return Ok(()),
            };

            if retries == 0 {
                // Return error
                return res;
            }
            retries -= 1;
            // Delay 10ms since the fuel gauge might be "thinking" from a previous command
            self.delay.delay_ms(DEFAULT_ERROR_BACKOFF_DELAY_MS).await;
        }
    }

    pub(crate) async fn write_with_retries(&mut self, write: &[u8]) -> Result<(), BQ40Z50Error<I2C::Error>> {
        self.write_with_retries_internal(write).await
    }

    pub(crate) async fn write_with_retries_pec(&mut self, write: &[u8]) -> Result<(), BQ40Z50Error<I2C::Error>> {
        // Buffer to hold the entire message to compute PEC on
        let mut pec_buf = [0u8; LARGEST_REG_SIZE_BYTES * 2];
        pec_buf[0] = BQ_ADDR << 1;
        pec_buf[1..=write.len()].copy_from_slice(write);

        // Write one more byte (PEC)
        let mut write_buf = [0u8; 1 + LARGEST_REG_SIZE_BYTES];
        write_buf[..write.len()].copy_from_slice(write);
        write_buf[write.len()] = smbus_pec::pec(&pec_buf[..=write.len()]);

        // Include everything we want to write plus the PEC byte
        let write_buf = &write_buf[..=write.len()];

        self.write_with_retries_internal(write_buf).await
    }

    pub(crate) async fn read_with_retries(
        &mut self,
        write: &[u8],
        read: &mut [u8],
    ) -> Result<(), BQ40Z50Error<I2C::Error>> {
        let mut retries = self.config.max_bus_retries;

        loop {
            let res = match with_timeout(self.config.timeout, self.i2c.write_read(BQ_ADDR, write, read)).await {
                Err(_) => Err(BQ40Z50Error::Timeout),
                Ok(Err(bus_err)) => Err(BQ40Z50Error::I2c(bus_err)),
                Ok(Ok(())) => return Ok(()),
            };

            if retries == 0 {
                // Return error
                return res;
            }
            retries -= 1;
            // Delay 10ms since the fuel gauge might be "thinking" from a previous command
            self.delay.delay_ms(DEFAULT_ERROR_BACKOFF_DELAY_MS).await;
        }
    }

    pub(crate) async fn read_with_retries_pec(
        &mut self,
        write: &[u8],
        read: &mut [u8],
    ) -> Result<(), BQ40Z50Error<I2C::Error>> {
        let mut retries = self.config.max_bus_retries;

        // Buffer to hold the entire message, including write and read, to compute PEC on
        let mut pec_buf = [0u8; LARGEST_REG_SIZE_BYTES * 2];
        pec_buf[0] = BQ_ADDR << 1;
        pec_buf[1..=write.len()].copy_from_slice(write);
        // Device Addr + Read Bit (1)
        pec_buf[write.len() + 1] = BQ_ADDR << 1 | 0x01;

        // Read one more byte (PEC)
        let mut read_buf = [0u8; 1 + LARGEST_REG_SIZE_BYTES];
        let read_buf = &mut read_buf[..=read.len()];

        loop {
            let res = match with_timeout(self.config.timeout, self.i2c.write_read(BQ_ADDR, write, read_buf)).await {
                Err(_) => Err(BQ40Z50Error::Timeout),
                Ok(Err(bus_err)) => Err(BQ40Z50Error::I2c(bus_err)),
                Ok(Ok(())) => {
                    let recvd_pec = read_buf[read.len()];
                    // Copy just read bytes to pec_buf, without the PEC byte
                    pec_buf[2 + write.len()..2 + write.len() + read.len()].copy_from_slice(&read_buf[..read.len()]);

                    if recvd_pec == smbus_pec::pec(&pec_buf[..2 + write.len() + read.len()]) {
                        // If all is good, copy bytes we read into read.
                        read.copy_from_slice(&read_buf[..read.len()]);
                        return Ok(());
                    }
                    Err(BQ40Z50Error::Pec)
                }
            };

            if retries == 0 {
                // Return error
                return res;
            }
            retries -= 1;
            // Delay 10ms since the fuel gauge might be "thinking" from a previous command
            self.delay.delay_ms(DEFAULT_ERROR_BACKOFF_DELAY_MS).await;
        }
    }

    pub(crate) async fn mac_read_with_retries(
        &mut self,
        write: &[u8],
        read: &mut [u8],
    ) -> Result<(), BQ40Z50Error<I2C::Error>> {
        let mut retries = self.config.max_bus_retries;

        // Loop until no bus errors or max bus retries are hit.
        loop {
            // Block write intended register.
            let res = match with_timeout(self.config.timeout, self.i2c.write(BQ_ADDR, write)).await {
                Err(_) => Err(BQ40Z50Error::Timeout),
                Ok(Err(bus_err)) => Err(BQ40Z50Error::I2c(bus_err)),
                Ok(Ok(())) => Ok(()),
            };

            if res.is_err() {
                if retries == 0 {
                    return res;
                }
                self.delay.delay_ms(DEFAULT_ERROR_BACKOFF_DELAY_MS).await;
                retries -= 1;
                continue;
            }

            // For read only commands.
            // Block read using I2C write_read, sending 0x44 as the command.
            // Response looks like [ Length (1 byte) | Command (2 bytes) | Data (read.len() bytes)]
            let mut output_buf = [0u8; 1 + LARGEST_CMD_SIZE_BYTES + MAC_CMD_ADDR_SIZE_BYTES as usize];

            let res = match with_timeout(
                self.config.timeout,
                self.i2c.write_read(
                    BQ_ADDR,
                    &[write[0]],
                    &mut output_buf[..1 + MAC_CMD_ADDR_SIZE_BYTES as usize + read.len()],
                ),
            )
            .await
            {
                Err(_) => Err(BQ40Z50Error::Timeout),
                Ok(Err(bus_err)) => Err(BQ40Z50Error::I2c(bus_err)),
                Ok(Ok(())) => Ok(()),
            };

            if res.is_err() {
                if retries == 0 {
                    return res;
                }
                self.delay.delay_ms(DEFAULT_ERROR_BACKOFF_DELAY_MS).await;
                retries -= 1;
                continue;
            }

            read.copy_from_slice(
                &output_buf
                    [(1 + MAC_CMD_ADDR_SIZE_BYTES as usize)..(1 + MAC_CMD_ADDR_SIZE_BYTES as usize + read.len())],
            );

            return Ok(());
        }
    }

    #[allow(clippy::range_plus_one)]
    pub(crate) async fn mac_read_with_retries_pec(
        &mut self,
        write: &[u8],
        read: &mut [u8],
    ) -> Result<(), BQ40Z50Error<I2C::Error>> {
        let mut retries = self.config.max_bus_retries;

        // Buffer to hold the entire message to compute PEC on
        let mut pec_buf = [0u8; 2 + LARGEST_CMD_SIZE_BYTES + 2 + MAC_CMD_ADDR_SIZE_BYTES as usize];
        pec_buf[0] = BQ_ADDR << 1;
        pec_buf[1..write.len() + 1].copy_from_slice(write);

        // Compute PEC for the Write Block
        // Write one more byte (PEC)
        // [ MAC_CMD (0x44) | CMD_SIZE | CMD_LSB | CMD_MSB | PEC ]
        let mut write_buf = [0u8; 1 + 2 + MAC_CMD_ADDR_SIZE_BYTES as usize];
        write_buf[..write.len()].copy_from_slice(write);
        write_buf[write.len()] = smbus_pec::pec(&pec_buf[..write.len() + 1]);

        // Include everything we want to write plus the PEC byte
        let write_buf = &write_buf[..=write.len()];

        // Read one more byte (PEC)
        let mut read_buf = [0u8; 1 + MAC_CMD_ADDR_SIZE_BYTES as usize + LARGEST_CMD_SIZE_BYTES];
        let read_buf = &mut read_buf[..1 + MAC_CMD_ADDR_SIZE_BYTES as usize + read.len() + 1];

        // Reuse pec_buf for the block read now that we've computed PEC for the block write.
        pec_buf[0] = BQ_ADDR << 1;
        pec_buf[1] = write_buf[0];
        pec_buf[2] = BQ_ADDR << 1 | 0x01;

        // Loop until no bus errors or max bus retries are hit.
        loop {
            // Block write intended register.
            let res = match with_timeout(self.config.timeout, self.i2c.write(BQ_ADDR, write_buf)).await {
                Err(_) => Err(BQ40Z50Error::Timeout),
                Ok(Err(bus_err)) => Err(BQ40Z50Error::I2c(bus_err)),
                Ok(Ok(())) => Ok(()),
            };

            if res.is_err() {
                if retries == 0 {
                    return res;
                }
                self.delay.delay_ms(DEFAULT_ERROR_BACKOFF_DELAY_MS).await;
                retries -= 1;
                continue;
            }

            // For read only commands.
            // Block read using I2C write_read, sending 0x44 as the command.
            // Response looks like [ Length (1 byte) | Command (2 bytes) | Data (output.len() bytes)]

            let res = match with_timeout(
                self.config.timeout,
                self.i2c.write_read(BQ_ADDR, &[write_buf[0]], read_buf),
            )
            .await
            {
                Err(_) => Err(BQ40Z50Error::Timeout),
                Ok(Err(bus_err)) => Err(BQ40Z50Error::I2c(bus_err)),
                Ok(Ok(())) => Ok(()),
            };

            if res.is_err() {
                if retries == 0 {
                    return res;
                }
                self.delay.delay_ms(DEFAULT_ERROR_BACKOFF_DELAY_MS).await;
                retries -= 1;
                continue;
            }

            let recvd_pec = read_buf[1 + MAC_CMD_ADDR_SIZE_BYTES as usize + read.len()];
            // Copy just read bytes to pec_buf, without the PEC byte
            pec_buf[3..3 + 1 + MAC_CMD_ADDR_SIZE_BYTES as usize + read.len()]
                .copy_from_slice(&read_buf[..1 + MAC_CMD_ADDR_SIZE_BYTES as usize + read.len()]);

            // Check PEC
            if recvd_pec != smbus_pec::pec(&pec_buf[..3 + 1 + MAC_CMD_ADDR_SIZE_BYTES as usize + read.len()]) {
                if retries == 0 {
                    return Err(BQ40Z50Error::Pec);
                }
                retries -= 1;
                // Delay 10ms since the fuel gauge might be "thinking" from a previous command
                self.delay.delay_ms(DEFAULT_ERROR_BACKOFF_DELAY_MS).await;
                continue;
            }

            read.copy_from_slice(
                &read_buf[(1 + MAC_CMD_ADDR_SIZE_BYTES as usize)..(1 + MAC_CMD_ADDR_SIZE_BYTES as usize + read.len())],
            );

            return Ok(());
        }
    }

    pub(crate) async fn mac_read_from_df_with_retries(
        &mut self,
        starting_address: u16,
        read: &mut [u8],
    ) -> Result<(), BQ40Z50Error<I2C::Error>> {
        let mut retries = self.config.max_bus_retries;
        let starting_address = starting_address.to_le_bytes();

        // Loop until no bus errors or max bus retries are hit.
        loop {
            // Block write intended register.
            let res = match with_timeout(
                self.config.timeout,
                self.i2c.write(
                    BQ_ADDR,
                    &[
                        MAC_CMD,
                        MAC_CMD_ADDR_SIZE_BYTES,
                        starting_address[0],
                        starting_address[1],
                    ],
                ),
            )
            .await
            {
                Err(_) => Err(BQ40Z50Error::Timeout),
                Ok(Err(bus_err)) => Err(BQ40Z50Error::I2c(bus_err)),
                Ok(Ok(())) => Ok(()),
            };

            if res.is_err() {
                if retries == 0 {
                    return res;
                }
                self.delay.delay_ms(DEFAULT_ERROR_BACKOFF_DELAY_MS).await;
                retries -= 1;
                continue;
            }

            // Read in 32 byte chunks. The FG supports an auto-increment on the address during a DF read.
            // If an SMBus read block is sent, the gauge will return 32 bytes of DF data,
            // and if a subsequent SMBus read block is sent with command 0x44,
            // the gauge returns another 32 bytes of DF data starting at the starting address + 32.
            let mut bytes_left_to_read = read.len();
            while bytes_left_to_read > 0 {
                // Largest single read block is 2 bytes starting address + 32 bytes data.
                let mut output_buf = [0u8; LARGEST_DF_BLOCK_SIZE_BYTES + MAC_CMD_ADDR_SIZE_BYTES as usize];
                // Determine how many bytes to read from the bus, ideally we want to minimize time reading from DF
                // so if we can read less than 32 bytes of DF data, do it.
                let output_buf_end_idx =
                    core::cmp::min(output_buf.len(), bytes_left_to_read + MAC_CMD_ADDR_SIZE_BYTES as usize);

                let res = match with_timeout(
                    self.config.timeout,
                    self.i2c
                        .write_read(BQ_ADDR, &[MAC_CMD], &mut output_buf[..output_buf_end_idx]),
                )
                .await
                {
                    Err(_) => Err(BQ40Z50Error::Timeout),
                    Ok(Err(bus_err)) => Err(BQ40Z50Error::I2c(bus_err)),
                    Ok(Ok(())) => Ok(()),
                };

                if res.is_err() {
                    if retries == 0 {
                        return res;
                    }
                    self.delay.delay_ms(DEFAULT_ERROR_BACKOFF_DELAY_MS).await;
                    retries -= 1;
                    continue;
                }

                let start_idx = read.len() - bytes_left_to_read;
                let end_idx = start_idx + output_buf_end_idx - MAC_CMD_ADDR_SIZE_BYTES as usize;
                read[start_idx..end_idx]
                    .copy_from_slice(&output_buf[(MAC_CMD_ADDR_SIZE_BYTES as usize)..output_buf_end_idx]);
                bytes_left_to_read = bytes_left_to_read.saturating_sub(LARGEST_DF_BLOCK_SIZE_BYTES);
            }

            return Ok(());
        }
    }

    pub(crate) async fn mac_read_from_df_with_retries_pec(
        &mut self,
        starting_address: u16,
        read: &mut [u8],
    ) -> Result<(), BQ40Z50Error<I2C::Error>> {
        let mut retries = self.config.max_bus_retries;
        let starting_address = starting_address.to_le_bytes();

        let pec = smbus_pec::pec(&[
            BQ_ADDR << 1,
            MAC_CMD,
            MAC_CMD_ADDR_SIZE_BYTES,
            starting_address[0],
            starting_address[1],
        ]);

        // Loop until no bus errors or max bus retries are hit.
        loop {
            // Block write intended register.
            let res = match with_timeout(
                self.config.timeout,
                self.i2c.write(
                    BQ_ADDR,
                    &[
                        MAC_CMD,
                        MAC_CMD_ADDR_SIZE_BYTES,
                        starting_address[0],
                        starting_address[1],
                        pec,
                    ],
                ),
            )
            .await
            {
                Err(_) => Err(BQ40Z50Error::Timeout),
                Ok(Err(bus_err)) => Err(BQ40Z50Error::I2c(bus_err)),
                Ok(Ok(())) => Ok(()),
            };

            if res.is_err() {
                if retries == 0 {
                    return res;
                }
                self.delay.delay_ms(DEFAULT_ERROR_BACKOFF_DELAY_MS).await;
                retries -= 1;
                continue;
            }

            // Read in 32 byte chunks. The FG supports an auto-increment on the address during a DF read.
            // If an SMBus read block is sent, the gauge will return 32 bytes of DF data,
            // and if a subsequent SMBus read block is sent with command 0x44,
            // the gauge returns another 32 bytes of DF data starting at the starting address + 32.
            let mut bytes_left_to_read = read.len();
            while bytes_left_to_read > 0 {
                // Largest single read block is 2 bytes starting address + 32 bytes data + 1 PEC byte.

                let mut output_buf = [0u8; LARGEST_DF_BLOCK_SIZE_BYTES + MAC_CMD_ADDR_SIZE_BYTES as usize + 1];
                // Determine how many bytes to read from the bus, ideally we want to minimize time reading from DF
                // so if we can read less than 32 bytes of DF data, do it.
                let output_buf_end_idx = core::cmp::min(
                    output_buf.len(),
                    bytes_left_to_read + MAC_CMD_ADDR_SIZE_BYTES as usize + 1,
                );

                let res = match with_timeout(
                    self.config.timeout,
                    self.i2c
                        .write_read(BQ_ADDR, &[MAC_CMD], &mut output_buf[..output_buf_end_idx]),
                )
                .await
                {
                    Err(_) => Err(BQ40Z50Error::Timeout),
                    Ok(Err(bus_err)) => Err(BQ40Z50Error::I2c(bus_err)),
                    Ok(Ok(())) => Ok(()),
                };

                if res.is_err() {
                    if retries == 0 {
                        return res;
                    }
                    self.delay.delay_ms(DEFAULT_ERROR_BACKOFF_DELAY_MS).await;
                    retries -= 1;
                    continue;
                }

                let recvd_pec = output_buf[output_buf_end_idx - 1];
                let mut pec = smbus_pec::Pec::new();
                pec.write(&[BQ_ADDR << 1, MAC_CMD, BQ_ADDR << 1 | 0x01]);
                // Omit PEC
                pec.write(&output_buf[..output_buf_end_idx - 1]);
                let pec = pec.finish();

                if u64::from(recvd_pec) != pec {
                    if retries == 0 {
                        return Err(BQ40Z50Error::Pec);
                    }
                    self.delay.delay_ms(DEFAULT_ERROR_BACKOFF_DELAY_MS).await;
                    retries -= 1;
                    continue;
                }

                let start_idx = read.len() - bytes_left_to_read;
                let end_idx = start_idx + output_buf_end_idx - MAC_CMD_ADDR_SIZE_BYTES as usize - 1;
                read[start_idx..end_idx]
                    .copy_from_slice(&output_buf[(MAC_CMD_ADDR_SIZE_BYTES as usize)..output_buf_end_idx - 1]);
                bytes_left_to_read = bytes_left_to_read.saturating_sub(LARGEST_DF_BLOCK_SIZE_BYTES);
            }

            return Ok(());
        }
    }
}

impl<I2C: I2cTrait, DELAY: DelayTrait> device_driver::AsyncRegisterInterface for DeviceInterface<I2C, DELAY> {
    type Error = BQ40Z50Error<I2C::Error>;
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

        if self.config.pec_write {
            self.write_with_retries_pec(&buf[..=data.len()]).await
        } else {
            self.write_with_retries(&buf[..=data.len()]).await
        }
    }

    async fn read_register(
        &mut self,
        address: Self::AddressType,
        _size_bits: u32,
        data: &mut [u8],
    ) -> Result<(), Self::Error> {
        if self.config.pec_read {
            self.read_with_retries_pec(&[address], data).await
        } else {
            self.read_with_retries(&[address], data).await
        }
    }
}

impl<I2C: I2cTrait, DELAY: DelayTrait> device_driver::AsyncCommandInterface for DeviceInterface<I2C, DELAY> {
    type Error = BQ40Z50Error<I2C::Error>;
    type AddressType = u32;

    async fn dispatch_command(
        &mut self,
        address: Self::AddressType,
        size_bits_in: u32,
        _input: &[u8],
        size_bits_out: u32,
        output: &mut [u8],
    ) -> Result<(), Self::Error> {
        // For this driver, dispatch_command() is used for interfacing with MAC registers.
        // There are 3 possible scenarios, read only, write only, or read/write registers.
        // Read commands have an output size but no input size.
        // Write commands do not have an input size nor output size since they are pure commands.
        // Read/write commands, like Security Keys and Authentication Key are special cases
        // and are handled on a per-register basis not within this function.

        // Block write first to send a command (write only commands) or to read command data from the fuel gauge
        let mut buf = [0u8; 2 + MAC_CMD_ADDR_SIZE_BYTES as usize];
        buf[0] = ((address >> MAC_CMD_ADDR_SIZE_BITS) & 0xFF) as u8;
        buf[1] = MAC_CMD_ADDR_SIZE_BYTES;
        buf[2] = ((address >> 8) & 0xFF) as u8;
        buf[3] = (address & 0xFF) as u8;

        if size_bits_in == 0 && size_bits_out == 0 {
            // Write only, writes don't have an output size nor an input size because
            // writes only consist of the register/command address.
            if self.config.pec_write {
                self.mac_write_with_retries_pec(&buf).await?;
            } else {
                self.mac_write_with_retries(&buf).await?;
            }
        } else if size_bits_in == 0 && size_bits_out > 0 {
            // For read only commands.
            if self.config.pec_read {
                self.mac_read_with_retries_pec(&buf, output).await?;
            } else {
                self.mac_read_with_retries(&buf, output).await?;
            }
        } else {
            // Read/write, to be handled in other functions as special cases.
            unreachable!();
        }
        Ok(())
    }
}

impl<I2C: I2cTrait, DELAY: DelayTrait> device_driver::BufferInterfaceError for DeviceInterface<I2C, DELAY> {
    type Error = BQ40Z50Error<I2C::Error>;
}

impl<I2C: I2cTrait, DELAY: DelayTrait> device_driver::AsyncBufferInterface for DeviceInterface<I2C, DELAY> {
    type AddressType = u8;

    async fn read(&mut self, address: Self::AddressType, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.read_with_retries(&[address], buf).await.map(|()| buf.len())
    }

    async fn write(&mut self, address: Self::AddressType, buf: &[u8]) -> Result<usize, Self::Error> {
        debug_assert!((buf.len() <= LARGEST_BUF_SIZE_BYTES), "Buffer size too big");

        // Add one byte for register address
        let mut data = [0u8; 1 + LARGEST_BUF_SIZE_BYTES];
        data[0] = address;
        data[1..=buf.len()].copy_from_slice(buf);

        self.write_with_retries(&data).await.map(|()| buf.len())
    }

    async fn flush(&mut self, _address: Self::AddressType) -> Result<(), Self::Error> {
        Ok(())
    }
}
