pub(crate) const LARGEST_REG_SIZE_BYTES: usize = 5;
pub(crate) const LARGEST_CMD_SIZE_BYTES: usize = 32;
pub(crate) const LARGEST_BUF_SIZE_BYTES: usize = 33;
pub(crate) const LARGEST_DF_BLOCK_SIZE_BYTES: usize = 32;

pub(crate) const BQ_ADDR: u8 = 0x0Bu8;
pub(crate) const MAC_CMD_ADDR_SIZE_BYTES: u8 = 2;
pub(crate) const MAC_CMD_ADDR_SIZE_BITS: u8 = MAC_CMD_ADDR_SIZE_BYTES * 8;
pub(crate) const MAC_CMD: u8 = 0x44u8;

// Special case MAC commands
pub(crate) const SECURITY_KEYS_CMD: [u8; MAC_CMD_ADDR_SIZE_BYTES as usize] = 0x0035u16.to_le_bytes();
pub(crate) const SECURITY_KEYS_DATA_LEN_BYTES: u8 = 8;
pub(crate) const SECURITY_KEYS_LEN_BYTES: u8 = SECURITY_KEYS_DATA_LEN_BYTES + MAC_CMD_ADDR_SIZE_BYTES;

pub(crate) const AUTH_KEY_CMD: [u8; MAC_CMD_ADDR_SIZE_BYTES as usize] = 0x0037u16.to_le_bytes();
pub(crate) const AUTH_KEY_DATA_LEN_BYTES: u8 = 16;
pub(crate) const AUTH_KEY_LEN_BYTES: u8 = AUTH_KEY_DATA_LEN_BYTES + MAC_CMD_ADDR_SIZE_BYTES;

pub(crate) const MFG_INFO_CMD: u8 = 0x70;

#[cfg(not(all(feature = "r1", not(any(feature = "r3", feature = "r4", feature = "r5")))))]
pub(crate) const CHRG_VOLTAGE_OVERRIDE_CMD: [u8; MAC_CMD_ADDR_SIZE_BYTES as usize] = 0x00B0u16.to_le_bytes();
#[cfg(not(all(feature = "r1", not(any(feature = "r3", feature = "r4", feature = "r5")))))]
pub(crate) const CHRG_VOLTAGE_OVERRIDE_SIZE_BYTES: u8 = 10;

pub(crate) const DEFAULT_BUS_RETRIES: usize = 3;
pub(crate) const DEFAULT_ERROR_BACKOFF_DELAY_MS: u32 = 10;
#[cfg(feature = "embassy-timeout")]
pub(crate) const DEFAULT_TIMEOUT: Duration = Duration::from_millis(100);

#[cfg(feature = "embassy-timeout")]
use embassy_time::Duration;
