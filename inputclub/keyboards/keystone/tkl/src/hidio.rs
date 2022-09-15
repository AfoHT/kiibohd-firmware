// Copyright 2021-2022 Jacob Alexander
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use super::constants::*;
use core::fmt::Write;
use heapless::String;
use keystonetkl::hal::chipid::ChipId;
use kiibohd_hid_io::*;

#[derive(defmt::Format)]
pub struct ManufacturingConfig {
    /// Cycles LEDs thruogh all available colors to check for dead LEDs
    pub led_test_sequence: bool,
    /// Lumissil LED short test
    pub led_short_test: bool,
    /// Lumissil LED open test
    pub led_open_test: bool,
    /// Hall Effect detect default level test (pass/fail)
    pub hall_pass_fail_test: bool,
    /// Hall Effect level check
    pub hall_level_check: bool,
}

pub struct HidioInterface<const H: usize> {
    mcu: Option<String<12>>,
    serial: Option<String<126>>,
    pub manufacturing_config: ManufacturingConfig,
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
            hall_pass_fail_test: false,
            hall_level_check: false,
        };

        Self {
            mcu,
            serial,
            manufacturing_config,
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
            // Hall Effect tests
            h0050::Command::HallEffectSensorTest => {
                match unsafe { data.argument.hall_effect_sensor_test } {
                    // Disables
                    h0050::args::HallEffectSensorTest::DisableAll => {
                        self.manufacturing_config.hall_pass_fail_test = false;
                        self.manufacturing_config.hall_level_check = false;
                        Ok(h0050::Ack {})
                    }
                    // Enables pass/fail test
                    // Sends data using h0051
                    h0050::args::HallEffectSensorTest::PassFailTestToggle => {
                        self.manufacturing_config.hall_pass_fail_test = true;
                        Ok(h0050::Ack {})
                    }
                    // Enables level check mode
                    // Sends data using h0051
                    h0050::args::HallEffectSensorTest::LevelCheckToggle => {
                        self.manufacturing_config.hall_level_check = true;
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
