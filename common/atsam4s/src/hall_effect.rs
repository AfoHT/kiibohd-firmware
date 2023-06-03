// Copyright 2021-2023 Jacob Alexander
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::constants::*;
use crate::*;
pub use kiibohd_hall_effect_keyscanning::lookup::{
    SILO_ATSAM4S_LC605_GAIN_2X, SILO_ATSAM4S_LC605_GAIN_4X,
};
pub use kiibohd_hall_effect_keyscanning::{Matrix, SensorMode};

use hal::{
    adc::{
        Adc, AdcPayload, SettlingTime, SingleEndedGain, SingleSequence, StartupTime, TrackingTime,
        TransferTime,
    },
    clock::{Enabled, Tc0Clock},
    pac::TC0,
    pdc::{ReadDmaPaused, RxDma, Transfer, W},
    timer::TimerCounterChannel,
};

// ----- Types -----

pub type AdcTransfer<const ADC_BUF_SIZE: usize> =
    Transfer<W, &'static mut [u16; ADC_BUF_SIZE], RxDma<AdcPayload<SingleSequence>>>;
pub type HallMatrix<const CSIZE: usize, const MSIZE: usize> =
    Matrix<PioX<Output<PushPull>>, CSIZE, MSIZE, INVERT_STROBE>;
pub type TCC0 = TimerCounterChannel<TC0, Tc0Clock<Enabled>, 0, TCC0_FREQ>;

// ----- Enums -----

/// Sets the ADC clock dynamically
///
/// Increasing the sample rate will decrease the required strobe time but may increase noise in the
/// readings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum AdcClock {
    /// 12 MHz
    Mhz12,
    /// 20 MHz
    Mhz20,
    /// 30 MHz
    Mhz30,
}

// ----- Structs -----

/// Container struct so it's easier to pass the sense pins around
/// Due to how the pin types are constructed it's currently not
/// possible to use an array like the strobe pins.
pub struct SensePins {
    pub sense1: Pa17<ExFn>,
    pub sense2: Pa18<ExFn>,
    pub sense3: Pa19<ExFn>,
    pub sense4: Pa20<ExFn>,
    pub sense5: Pa21<ExFn>,
    pub sense6: Pa22<ExFn>,
}

// ----- Initialization Functions -----

/// Initialize Hall Effect Matrix
/// Sets up ADC and GPIO pins
pub fn init<const CSIZE: usize, const RSIZE: usize, const MSIZE: usize>(
    adc: hal::pac::ADC,
    adc_clock: hal::clock::AdcClock<Enabled>,
    cols: [PioX<Output<PushPull>>; CSIZE],
    sense_pins: &mut SensePins,
    tcc0: &mut TCC0,
) -> (
    hal::adc::AdcDma<hal::adc::SingleSequence>,
    HallMatrix<CSIZE, MSIZE>,
) {
    // Setup hall effect matrix
    defmt::trace!("HE Matrix initialization");
    let mut matrix = HallMatrix::new(
        cols,
        DEFAULT_ADC_ANALYSIS_MODE,
        DEFAULT_ACTIVATION_DIST,
        DEFAULT_DEACTIVATION_DIST,
    )
    .unwrap();
    matrix.next_strobe().unwrap(); // Strobe first column

    // Setup ADC for hall effect matrix
    defmt::trace!("ADC initialization");
    let mut adc = Adc::new(adc, adc_clock);

    adc.enable_channel(&mut sense_pins.sense1);
    adc.enable_channel(&mut sense_pins.sense2);
    adc.enable_channel(&mut sense_pins.sense3);
    adc.enable_channel(&mut sense_pins.sense4);
    adc.enable_channel(&mut sense_pins.sense5);
    adc.enable_channel(&mut sense_pins.sense6);

    // Enabling all channels from 0 to 11 so we can use an 11 channel sequence
    // This also takes care of sense1 to sense6
    // XXX We must make sure not to enable too many channels, otherwise the sequence
    //     will be too long.
    for ch in 0..=11 {
        adc.enable_channel_id(ch);
    }

    // Interleave channel 11 (which is always disconnected) with the other channels to prevent ADC
    // cross-talk between channels.
    adc.sequence(&[0, 11, 1, 11, 2, 11, 3, 11, 8, 11, 9, 11]);
    adc.enable_sequencing();

    // Enable ADC tags (used to identify which channel the data is from; easier to debug channels)
    adc.enable_tags();

    // Keyscanning Timer
    tcc0.clock_input(TCC0_DIV);

    // Setup default analysis mode
    let mut adc = set_analysis_mode::<RSIZE>(
        DEFAULT_ADC_ANALYSIS_MODE,
        DEFAULT_ADC_CLOCK,
        adc,
        tcc0,
        sense_pins,
    );

    // Finalize ADC setup
    adc.enable_rxbuff_interrupt();
    let adc = adc.with_pdc();

    // Finalize timer setup
    defmt::trace!("TCC0 started - Keyscanning");
    tcc0.enable_interrupt();

    (adc, matrix)
}

/// Configures ADC + timer according to the analysis mode and sample rate
pub fn set_analysis_mode<const RSIZE: usize>(
    mode: SensorMode,
    adc_clock: AdcClock,
    adc: hal::adc::Adc,
    tcc0: &mut TCC0,
    sense_pins: &mut SensePins,
) -> hal::adc::Adc {
    // Set ADC timing
    let mut adc = match adc_clock {
        AdcClock::Mhz12 => adc_12mhz(adc),
        AdcClock::Mhz20 => adc_20mhz(adc),
        AdcClock::Mhz30 => adc_30mhz(adc),
    };

    // Set ADC levels
    let (gain, offset, timing) = match mode {
        // Non-optimized mode for testing, signficantly reduces sensitivity but allows for full
        // hall effect sensor range (both positive and negative polarities).
        SensorMode::Test(_) => {
            defmt::debug!("ADC Test mode");
            // TODO calculate minimum latency
            (
                SingleEndedGain::Gain2x,
                true,
                5000_u32.micros() / RSIZE as u32,
            )
        }
        // Configures ADC to be optimized for Input Club Silo switches and low latency
        SensorMode::LowLatency(_) => {
            defmt::debug!("ADC Low Latency mode");
            // Absolute minimum latency is 1 ms / total number of columns (RSIZE)
            // Consistency will be negatively affected if key polling rate doesn't match
            // the USB polling rate for USB 2.0 FS.
            (
                SingleEndedGain::Gain4x,
                true,
                1000_u32.micros() / RSIZE as u32,
            )
        }
        // Configures ADC to be optimized for Input Club Silo switches
        // Will not work if magnets are not with a specific strength range
        // Adds at least 10x more range compared to test_mode()
        SensorMode::Normal(_) => {
            defmt::debug!("ADC Full Analysis mode");
            // TODO calculate minimum latency
            (
                SingleEndedGain::Gain4x,
                true,
                2000_u32.micros() / RSIZE as u32,
            )
        }
    };

    // Gain
    adc.gain(&mut sense_pins.sense1, gain);
    adc.gain(&mut sense_pins.sense2, gain);
    adc.gain(&mut sense_pins.sense3, gain);
    adc.gain(&mut sense_pins.sense4, gain);
    adc.gain(&mut sense_pins.sense5, gain);
    adc.gain(&mut sense_pins.sense6, gain);

    // Offset
    adc.offset(&mut sense_pins.sense1, offset);
    adc.offset(&mut sense_pins.sense2, offset);
    adc.offset(&mut sense_pins.sense3, offset);
    adc.offset(&mut sense_pins.sense4, offset);
    adc.offset(&mut sense_pins.sense5, offset);
    adc.offset(&mut sense_pins.sense6, offset);

    // Autocalibration is needed after gain/offset setting changes
    adc.autocalibration(true);

    // Setup timer
    tcc0.start(timing);

    adc
}

/// Configures ADC timing for 12 MHz
fn adc_12mhz(mut adc: hal::adc::Adc) -> hal::adc::Adc {
    defmt::info!("ADC set to 12 MHz");

    adc.prescaler(1); // 12 MHz from 120 MHz

    // Startup time
    //     Startup time = 512 / 12MHz = 42.6 us
    adc.startup_time(StartupTime::Sut512);

    // Tracking time
    //     Ttrack minimum = 0.054 * Zsource + 205
    //     Ttrack minimum = 0.054 * 20k + 205 = 1285 ns
    //     12MHz -> 83.3 ns * 15 cycles = 1250 ns
    //
    //     Tracking Time = (1 + 1) / 12MHz = 167 ns
    //     1250 ns + 167 ns = 1417 ns
    //
    //     (Maximum Tracking Time with 12 MHz)
    //     Tracking Time = (15 + 1) / 12MHz = 1333.3 ns
    adc.tracking_time(TrackingTime::Tt2);

    // Transfer time
    adc.transfer_time(TransferTime::Tt7);

    // Settling time
    adc.settling_time(SettlingTime::Ast17);

    adc
}

/// Configures ADC timing for 20 MHz
fn adc_20mhz(mut adc: hal::adc::Adc) -> hal::adc::Adc {
    defmt::info!("ADC set to 20 MHz");

    adc.prescaler(2); // 20 MHz from 120 MHz

    // Startup time
    //     Startup time = 512 / 20MHz = 25.6 us
    adc.startup_time(StartupTime::Sut512);

    // Tracking time
    //     Ttrack minimum = 0.054 * Zsource + 205
    //     Ttrack minimum = 0.054 * 20k + 205 = 1285 ns
    //     20MHz -> 50 ns * 15 cycles = 750 ns
    //
    //     Tracking Time = (10 + 1) / 20MHz = 550 ns
    //     750 ns + 550 ns = 1300 ns
    //
    //     (Maximum Tracking Time with 20 MHz)
    //     Tracking Time = (15 + 1) / 20MHz = 800 ns
    //
    adc.tracking_time(TrackingTime::Tt11);

    // Transfer time
    adc.transfer_time(TransferTime::Tt7);

    // Settling time
    adc.settling_time(SettlingTime::Ast17);

    adc
}

/// Configures ADC timing for 30 MHz
/// XXX (HaaTa) - This is technically overclocking as the ADC supported maximum is 22 MHz
/// See (44.8.3) page 1174
/// <https://ww1.microchip.com/downloads/aemDocuments/documents/OTH/ProductDocuments/DataSheets/Atmel-11100-32-bitCortex-M4-Microcontroller-SAM4S_Datasheet.pdf>
fn adc_30mhz(mut adc: hal::adc::Adc) -> hal::adc::Adc {
    defmt::info!("Overclocking ADC to 30 MHz");

    adc.prescaler(1); // 30 MHz from 120 MHz

    // Startup time
    //     Startup time = 512 / 30MHz = 17.1 us
    adc.startup_time(StartupTime::Sut512);

    // Tracking time
    //     Ttrack minimum = 0.054 * Zsource + 205
    //     Ttrack minimum = 0.054 * 20k + 205 = 1285 ns
    //     30MHz -> 33.3 ns * 15 cycles = 500 ns
    //
    //     Tracking Time = (15 + 1) / 30MHz = 533.3 ns
    //     500 ns + 533 ns = 1033 ns
    //
    adc.tracking_time(TrackingTime::Tt16);

    // Transfer time
    adc.transfer_time(TransferTime::Tt7);

    // Settling time
    adc.settling_time(SettlingTime::Ast17);

    adc
}

// ----- Software Interrupt Tasks -----

// ----- IRQ Functions -----

/// ADC Interrupt
/// Returns the current strobe index, this will wrap-around to 0 after the last strobe
#[allow(clippy::too_many_arguments)]
pub fn adc_irq<
    const CSIZE: usize,
    const RSIZE: usize,
    const MSIZE: usize,
    const ADC_BUF_SIZE: usize,
>(
    adc_pdc: &mut Option<AdcTransfer<ADC_BUF_SIZE>>,
    sense_pins: &mut SensePins,
    mut sensor_mode: Option<SensorMode>,
    tcc0: &mut TCC0,
    hidio_intf: &mut HidioCommandInterface,
    layer_state: &mut LayerState,
    manu_test_data: &mut heapless::Vec<u8, { kiibohd_hid_io::MESSAGE_LEN - 4 }>,
    matrix: &mut HallMatrix<CSIZE, MSIZE>,
    switch_remap: &[u8],
) -> usize {
    // Current strobe
    let strobe = matrix.strobe();

    // The first byte is used to check if the buffer is not complete yet
    // 0 - Not ready
    // 1 - Ready (don't push anymore data)
    let collect_manu_strobe = manu_test_data[1];
    let collect_manu_test_data = manu_test_data[0] == 0 && collect_manu_strobe == strobe as u8;

    // Retrieve DMA buffer if ready
    // This only happens during the first strobe right after initialization
    if !adc_pdc.as_ref().unwrap().is_done() {
        return strobe;
    }
    let (buf, adc) = adc_pdc.take().unwrap().wait();

    // Manufacturing test data
    // Used to accumulate ADC data for manufacturing tests
    if collect_manu_test_data {
        manu_test_data.push(strobe as u8).unwrap(); // Current strobe in buffer
        manu_test_data.push(RSIZE as u8).unwrap(); // Size of buffer
                                                   // Extend the array for 6 (RSIZE) samples (each row)
                                                   // Each sample is u16 so this needs to be converted to be split
                                                   // This is necessary as samples may arrive out of order
                                                   // Use a value larger than 12-bits to indicate we didn't get a sample
        manu_test_data
            .resize(manu_test_data.len() + RSIZE * 2, 0xFF)
            .unwrap();
    }

    // Process retrieved ADC buffer
    // Loop through buffer, samples may arrive out of order in some situations
    // (usually due to timing being too aggressive).
    // For example, 12 entries + 6 rows, column 0:
    //  Col Row Channel Sample: Entry
    //    0   0       0      0   6 * 0 + 0 = 0
    //    0   -      11      1   N/A (crosstalk isolation)
    //    0   1       1      2   6 * 0 + 1 = 1
    //    0   -      11      3   N/A (crosstalk isolation)
    //    0   2       2      4   6 * 0 + 2 = 2
    //    0   -      11      5   N/A (crosstalk isolation)
    //    0   3       3      6   6 * 0 + 3 = 3
    //    0   -      11      7   N/A (crosstalk isolation)
    //    0   4       8      8   6 * 0 + 4 = 4
    //    0   -      11      9   N/A (crosstalk isolation)
    //    0   5       9      10  6 * 0 + 5 = 5
    //    0   -      11      11  N/A (crosstalk isolation)
    for (i, sample) in buf.iter().enumerate() {
        // Handle multiple samples from the same buffer
        let channel = (sample & 0xF000) >> 12;
        // Mask 12 bits and shift right 2 bits to get 10-bit sample
        let sample = (sample & 0x0FFF) >> 2;

        // Remap channels to rows
        let row = match channel {
            0..=3 => channel,
            8 => 4,
            9 => 5,
            _ => continue,
        } as usize;

        // Lookup switch index and record sample
        let index = row * CSIZE + strobe;
        match matrix.record::<IDLE_LIMIT>(index, sample) {
            Ok(val) => {
                // If sample is valid and sensor is calibrated, pass to the next stage.
                if let Some(sense) = val {
                    // Store data for manufacturing test results
                    if collect_manu_test_data {
                        let data_pos = manu_test_data.len() - RSIZE * 2;
                        for (i, byte) in sense.data().value().to_le_bytes().iter().enumerate() {
                            manu_test_data[data_pos + i + row * 2] = *byte;
                        }
                    }

                    // Generate KLL trigger events
                    for event in sense
                        .trigger_events::<MAX_PER_KEY_EVENTS>(switch_remap[index] as usize, false)
                    {
                        let hidio_event = HidIoEvent::TriggerEvent(event);

                        // Enqueue KLL trigger event
                        let ret = layer_state.process_trigger::<MAX_LAYER_LOOKUP_SIZE>(event);
                        assert!(ret.is_ok(), "Failed to enqueue: {:?} - {:?}", event, ret);

                        // Enqueue HID-IO trigger event
                        if let Err(err) = hidio_intf.process_event(hidio_event) {
                            defmt::error!("Hidio TriggerEvent Error: {:?}", err);
                        }
                    }
                }
            }
            Err(e) => {
                defmt::error!(
                    "Sample record failed ({}, {}, {}):{} -> {}",
                    i,
                    strobe,
                    index,
                    sample,
                    e
                );
            }
        }
    }

    // Strobe next column
    if let Ok(strobe) = matrix.next_strobe() {
        if collect_manu_test_data {
            // Set next strobe to collect
            manu_test_data[1] = strobe as u8;

            // Buffer is ready to send via hidio (full)
            if manu_test_data.len() + RSIZE * 2 + 2 >= manu_test_data.capacity() {
                manu_test_data[0] = 1; // Set buffer ready
            }
        }
    }

    // Change sensor mode
    let adc = if sensor_mode.is_some() {
        set_analysis_mode::<RSIZE>(
            sensor_mode.take().unwrap(),
            DEFAULT_ADC_CLOCK,
            adc.revert(),
            tcc0,
            sense_pins,
        )
        .with_pdc()
    } else {
        adc
    };

    // Prepare next DMA read, but don't start it yet
    adc_pdc.replace(adc.read_paused(buf));

    strobe
}

pub fn hidio_send_irq(
    hidio_intf: &mut HidioCommandInterface,
    manu_test_data: &mut heapless::Vec<u8, { kiibohd_hid_io::MESSAGE_LEN - 4 }>,
) {
    // If manufacturing test is enabled, buffer is ready, send accumulated data
    if manu_test_data[0] == 1
        && hidio_intf.interface().manufacturing_config.hall_level_check
        && hidio_intf
            .h0051_manufacturingres(h0051::Cmd {
                command: h0051::Command::HallEffectSensorTest,
                argument: h0051::Argument {
                    hall_effect_sensor_test: h0051::args::HallEffectSensorTest::LevelCheck,
                },
                data: manu_test_data.clone(),
            })
            .is_err()
    {
        defmt::warn!("Buffer full, failed to send hall level check data");
    }

    // Clear manufacturing test data
    manu_test_data.clear();
    let _ = manu_test_data.push(0);
}
