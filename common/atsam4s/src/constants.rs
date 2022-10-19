// Copyright 2021-2022 Jacob Alexander
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use atsam4_hal::timer::ClockSource;
use const_env::from_env;

// ----- Flash Config -----

pub const FLASH_CONFIG_SIZE: usize = 524288 / core::mem::size_of::<u32>();
extern "C" {
    #[link_name = "_flash"]
    pub static mut FLASH_CONFIG: [u32; FLASH_CONFIG_SIZE];
}

// ----- Constants -----

// General clock frequencies
pub const MCU_FREQ: u32 = 120_000_000;

// RTT frequency calculations
pub const RTT_PRESCALER: usize = 4; // Most accurate than evenly counts seconds

// Timer frequency calculations
pub const TCC0_DIV: ClockSource = ClockSource::MckDiv128;
pub const TCC0_FREQ: u32 = MCU_FREQ / TCC0_DIV.div();
pub const TCC1_DIV: ClockSource = ClockSource::MckDiv128;
pub const TCC1_FREQ: u32 = MCU_FREQ / TCC1_DIV.div();
pub const TCC2_FREQ: u32 = TCC1_FREQ;

pub const BUF_CHUNK: usize = 64;
pub const ID_LEN: usize = 10;
pub const RX_BUF: usize = 8;
pub const SERIALIZATION_LEN: usize = 277;
pub const TX_BUF: usize = 8;

pub const ADC_SAMPLES: usize = 2; // Number of samples per key per strobe
                                  // for the previous strobe's last sample)
pub const INVERT_STROBE: bool = true; // P-Mosfets need to be inverted
pub const ISSI_DRIVER_CHANNELS: usize = 198;
pub const ISSI_DRIVER_CHIPS: usize = 2;
pub const ISSI_DRIVER_QUEUE_SIZE: usize = 5;
pub const ISSI_DRIVER_CS_LAYOUT: [u8; ISSI_DRIVER_CHIPS] = [0, 1];
// Must be 256 or less, or a power of 2; e.g. 512 due limitations with embedded-dma
// Actual value should be -> ISSI_DRIVER_CHIPS * 198 (e.g. 396);
// Size is determined by the largest SPI tx transaction
pub const SPI_TX_BUF_SIZE: usize = 512;
// Size is determined by the largest SPI rx transaction
pub const SPI_RX_BUF_SIZE: usize = (32 + 2) * ISSI_DRIVER_CHIPS;

pub const CTRL_QUEUE_SIZE: usize = 5;
pub const KBD_QUEUE_SIZE: usize = 25;
pub const KBD_LED_QUEUE_SIZE: usize = 3;
pub const MOUSE_QUEUE_SIZE: usize = 10;

// Keyscanning Constants
pub const DEBOUNCE_US: u32 = 5000; // 5 ms TODO Tuning
pub const IDLE_MS: u32 = 600_000; // 600 seconds TODO Tuning

// KLL Constants
pub const LAYOUT_SIZE: usize = 256;
pub const MAX_ACTIVE_LAYERS: usize = 8;
pub const MAX_ACTIVE_TRIGGERS: usize = 64;
pub const MAX_LAYERS: usize = 16;
pub const MAX_LAYER_STACK_CACHE: usize = 64;
pub const MAX_LAYER_LOOKUP_SIZE: usize = 64;
pub const MAX_OFF_STATE_LOOKUP: usize = 16;
pub const STATE_SIZE: usize = 32;

#[from_env]
pub const VID: u16 = 0x1c11;
#[from_env]
pub const PID: u16 = 0xb04d;
#[from_env]
pub const USB_MANUFACTURER: &str = "Unknown";
#[from_env]
pub const USB_PRODUCT: &str = "Kiibohd";
#[from_env]
pub const HIDIO_DEVICE_NAME: &str = "Kiibohd";
#[from_env]
pub const HIDIO_DEVICE_VENDOR: &str = "Unknown";
#[from_env]
pub const HIDIO_FIRMWARE_NAME: &str = "kiibohd-firmware";
