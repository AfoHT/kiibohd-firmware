// Copyright 2021-2022 Jacob Alexander
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use super::constants::*;
use core::fmt::Write;
use gemini::hal::chipid::ChipId;
use heapless::{String, Vec};
use kiibohd_hid_io::*;

#[derive(defmt::Format)]
pub struct ManufacturingConfig {
    /// Cycles LEDs thruogh all available colors to check for dead LEDs
    pub led_test_sequence: bool,
    /// Lumissil LED short test
    pub led_short_test: bool,
    /// Lumissil LED open test
    pub led_open_test: bool,
    /// Switch shake test (cycles color of switch LED on each press and release event)
    pub shake_test_led_cycle: bool,
}

#[derive(defmt::Format)]
pub struct LedControl {
    /// Control mode of the LED driver
    pub control: h0021::args::Control,
    /// Whether to trigger a soft reset of the LED driver on the next update
    pub soft_reset: bool,
    /// Whether to trigger a hard reset of the LED driver on the next update
    pub hard_reset: bool,
    /// Iterate to the next frame when using EnablePause mode (ignored otherwise)
    pub next_frame: bool,
}

pub struct HidioInterface<const H: usize> {
    pub led_buffer: Vec<u8, { ISSI_DRIVER_CHIPS * ISSI_DRIVER_CHANNELS }>,
    pub led_control: LedControl,
    pub manufacturing_config: ManufacturingConfig,
    mcu: Option<String<12>>,
    serial: Option<String<126>>,
}

impl<const H: usize> HidioInterface<H> {
    pub fn new(chip: &ChipId, serial: Option<String<126>>) -> Self {
        let mcu = if let Some(model) = chip.model() {
            let mut mcu: String<12> = String::new();
            if write!(mcu, "{:?}", model).is_ok() {
                Some(mcu)
            } else {
                None
            }
        } else {
            None
        };

        // Default all tests to off
        let manufacturing_config = ManufacturingConfig {
            led_test_sequence: false,
            led_short_test: false,
            led_open_test: false,
            shake_test_led_cycle: false,
        };

        // Default to all controls disabled
        let led_control = LedControl {
            control: h0021::args::Control::Disable,
            soft_reset: false,
            hard_reset: false,
            next_frame: false,
        };

        let mut led_buffer = Vec::new();
        led_buffer
            .resize_default(ISSI_DRIVER_CHIPS * ISSI_DRIVER_CHANNELS)
            .unwrap();

        Self {
            led_buffer,
            led_control,
            manufacturing_config,
            mcu,
            serial,
        }
    }
}

impl<const H: usize> KiibohdCommandInterface<H> for HidioInterface<H> {
    fn h0001_device_name(&self) -> Option<&str> {
        Some(HIDIO_DEVICE_NAME)
    }

    fn h0001_device_mcu(&self) -> Option<&str> {
        if let Some(mcu) = &self.mcu {
            Some(mcu)
        } else {
            None
        }
    }

    fn h0001_device_serial_number(&self) -> Option<&str> {
        if let Some(serial) = &self.serial {
            Some(serial)
        } else {
            None
        }
    }

    fn h0001_device_vendor(&self) -> Option<&str> {
        Some(HIDIO_DEVICE_VENDOR)
    }

    fn h0001_firmware_name(&self) -> Option<&str> {
        Some(HIDIO_FIRMWARE_NAME)
    }

    fn h0001_firmware_version(&self) -> Option<&str> {
        Some(VERGEN_GIT_SEMVER)
    }

    fn h0021_pixelsetting_cmd(&mut self, data: h0021::Cmd) -> Result<h0021::Ack, h0021::Nak> {
        defmt::info!("h0021_pixelsetting_cmd: {:?}", data);
        match data.command {
            h0021::Command::Control => {
                self.led_control.control = unsafe { data.argument.control };
            }
            h0021::Command::Reset => match unsafe { data.argument.reset } {
                h0021::args::Reset::SoftReset => {
                    self.led_control.soft_reset = true;
                }
                h0021::args::Reset::HardReset => {
                    self.led_control.hard_reset = true;
                }
            },
            h0021::Command::Clear => match unsafe { data.argument.clear } {
                h0021::args::Clear::Clear => {
                    self.led_buffer.clear();
                    self.led_buffer
                        .resize_default(ISSI_DRIVER_CHIPS * ISSI_DRIVER_CHANNELS)
                        .unwrap();
                }
            },
            h0021::Command::Frame => match unsafe { data.argument.frame } {
                h0021::args::Frame::NextFrame => {
                    self.led_control.next_frame = true;
                }
            },
            _ => {
                return Err(h0021::Nak {});
            }
        }
        Ok(h0021::Ack {})
    }

    fn h0026_directset_cmd(
        &mut self,
        data: h0026::Cmd<{ MESSAGE_LEN - 2 }>,
    ) -> Result<h0026::Ack, h0026::Nak> {
        defmt::info!("h0026_directset_cmd: {:?}", data);
        // Make sure hid-io control is enabled
        if self.led_control.control == h0021::args::Control::Disable {
            defmt::warn!("h0026_directset_cmd: hid-io control is disabled");
            return Err(h0026::Nak {});
        }

        // Make sure the buffer is large enough
        if self.led_buffer.len() < data.data.len() {
            defmt::warn!("h0026_directset_cmd: buffer too small");
            return Err(h0026::Nak {});
        }

        // Copy the data into the buffer from the starting address
        self.led_buffer[data.start_address as usize..data.data.len()].copy_from_slice(&data.data);

        Ok(h0026::Ack {})
    }

    fn h0050_manufacturing_cmd(&mut self, data: h0050::Cmd) -> Result<h0050::Ack, h0050::Nak> {
        // Make sure these are valid command/arguments for this keyboard
        let ret = match data.command {
            // LED test sequences
            h0050::Command::LedTestSequence => {
                match unsafe { data.argument.led_test_sequence } {
                    // Disable all
                    h0050::args::LedTestSequence::Disable => {
                        self.manufacturing_config.led_test_sequence = false;
                        self.manufacturing_config.led_short_test = false;
                        self.manufacturing_config.led_open_test = false;
                        Ok(h0050::Ack {})
                    }
                    // Toggle LED test sequence
                    h0050::args::LedTestSequence::Enable => {
                        self.manufacturing_config.led_test_sequence = true;
                        Ok(h0050::Ack {})
                    }
                    // Enable LED short test (auto disable after completion)
                    // Sends data using h0051
                    h0050::args::LedTestSequence::ActivateLedShortTest => {
                        self.manufacturing_config.led_short_test = true;
                        Ok(h0050::Ack {})
                    }
                    // Enable LED open test (auto disable after completion)
                    // Sends data using h0051
                    h0050::args::LedTestSequence::ActivateLedOpenCircuitTest => {
                        self.manufacturing_config.led_open_test = true;
                        Ok(h0050::Ack {})
                    }
                }
            }
            // Shake test
            h0050::Command::LedCycleKeypressTest => {
                match unsafe { data.argument.led_cycle_keypress_test } {
                    // Disables
                    h0050::args::LedCycleKeypressTest::Disable => {
                        self.manufacturing_config.shake_test_led_cycle = false;
                        Ok(h0050::Ack {})
                    }
                    // Enables shake test
                    h0050::args::LedCycleKeypressTest::Enable => {
                        self.manufacturing_config.shake_test_led_cycle = true;
                        Ok(h0050::Ack {})
                    }
                }
            }
            _ => Err(h0050::Nak {}),
        };
        defmt::trace!(
            "h0050_manufacturing_cmd: {:?} -> {:?}",
            data,
            self.manufacturing_config
        );
        ret
    }
}
