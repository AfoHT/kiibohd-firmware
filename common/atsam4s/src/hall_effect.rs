// Copyright 2021-2022 Jacob Alexander
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::constants::*;
use crate::*;
pub use kiibohd_hall_effect_keyscanning;

use hal::{
    adc::{Adc, AdcPayload, Continuous, SingleEndedGain},
    pdc::{ReadDmaPaused, RxDma, Transfer, W},
};

// ----- Types -----

pub type AdcTransfer<const ADC_BUF_SIZE: usize> =
    Transfer<W, &'static mut [u16; ADC_BUF_SIZE], RxDma<AdcPayload<Continuous>>>;
pub type HallMatrix<const CSIZE: usize, const MSIZE: usize> =
    kiibohd_hall_effect_keyscanning::Matrix<PioX<Output<PushPull>>, CSIZE, MSIZE, INVERT_STROBE>;

// ----- Initialization Functions -----

/// Initialize Hall Effect Matrix
/// Sets up ADC and GPIO pins
#[allow(clippy::too_many_arguments)]
pub fn init<const CSIZE: usize, const MSIZE: usize>(
    adc: hal::pac::ADC,
    adc_clock: hal::clock::AdcClock<Enabled>,
    cols: [PioX<Output<PushPull>>; CSIZE],
    sense1: &mut Pa17<ExFn>,
    sense2: &mut Pa18<ExFn>,
    sense3: &mut Pa19<ExFn>,
    sense4: &mut Pa20<ExFn>,
    sense5: &mut Pa21<ExFn>,
    sense6: &mut Pa22<ExFn>,
    tc0_chs: &mut TimerCounterChannels,
) -> (
    hal::adc::AdcDma<hal::adc::Continuous>,
    HallMatrix<CSIZE, MSIZE>,
) {
    // Setup hall effect matrix
    defmt::trace!("HE Matrix initialization");
    let mut matrix = HallMatrix::new(cols).unwrap();
    matrix.next_strobe().unwrap(); // Strobe first column

    // Setup ADC for hall effect matrix
    defmt::trace!("ADC initialization");
    let gain = SingleEndedGain::Gain4x;
    let offset = true;
    let mut adc = Adc::new(adc, adc_clock);

    adc.enable_channel(sense1);
    adc.enable_channel(sense2);
    adc.enable_channel(sense3);
    adc.enable_channel(sense4);
    adc.enable_channel(sense5);
    adc.enable_channel(sense6);

    adc.gain(sense1, gain);
    adc.gain(sense2, gain);
    adc.gain(sense3, gain);
    adc.gain(sense4, gain);
    adc.gain(sense5, gain);
    adc.gain(sense6, gain);

    adc.offset(sense1, offset);
    adc.offset(sense2, offset);
    adc.offset(sense3, offset);
    adc.offset(sense4, offset);
    adc.offset(sense5, offset);
    adc.offset(sense6, offset);

    adc.enable_tags();
    adc.autocalibration(true);
    adc.enable_rxbuff_interrupt();
    let adc = adc.with_continuous_pdc();

    // Keyscanning Timer
    let tcc0 = &mut tc0_chs.ch0;
    tcc0.clock_input(TCC0_DIV);
    tcc0.start(200_u32.micros());
    defmt::trace!("TCC0 started - Keyscanning");
    tcc0.enable_interrupt();

    (adc, matrix)
}

// ----- Software Interrupt Tasks -----

// ----- IRQ Functions -----

/// ADC Interrupt
/// Returns the current strobe index, this will wrap-around to 0 after the last strobe
pub fn adc_irq<
    const CSIZE: usize,
    const RSIZE: usize,
    const MSIZE: usize,
    const ADC_BUF_SIZE: usize,
>(
    adc_pdc: &mut Option<AdcTransfer<ADC_BUF_SIZE>>,
    hidio_intf: &mut HidioCommandInterface,
    layer_state: &mut LayerState,
    manu_test_data: &mut heapless::Vec<u8, { kiibohd_hid_io::MESSAGE_LEN - 4 }>,
    matrix: &mut HallMatrix<CSIZE, MSIZE>,
    strobe_cycle: &mut u32,
    switch_remap: &[u8],
) -> usize {
    // Determine if we should collect manufacturing data
    // There is too much data to send after every scan cycle (overwhelms HID-IO and USB)
    let collect_manu_test = *strobe_cycle % 25 == 0;

    // Retrieve DMA buffer
    let (buf, adc) = adc_pdc.take().unwrap().wait();
    //defmt::trace!("DMA BUF: {}", buf);

    // Current strobe
    let strobe = matrix.strobe();

    // Manufacturing test data
    // Used to accumulate ADC data for manufacturing tests
    if collect_manu_test {
        manu_test_data.push(strobe as u8).unwrap(); // Current strobe in buffer
        manu_test_data.push(RSIZE as u8).unwrap(); // Size of buffer
    }

    // Process retrieved ADC buffer
    // Loop through buffer. The buffer may have multiple buffers for each key.
    // For example, 13 entries + 6 rows, column 1:
    //  Col Row Channel Sample: Entry
    //    1   5       9      0  N/A (ignored, previous column reading)
    //    1   0       0      1  6 * 1 + 1 = 7
    //    1   1       1      2  6 * 1 + 2 = 8
    //    1   2       2      3  6 * 1 + 3 = 9
    //    1   3       3      4  6 * 1 + 4 = 10
    //    1   4       8      5  6 * 1 + 5 = 11
    //    1   5       9      6  6 * 1 + 0 = 6
    //    1   0       0      7  6 * 1 + 1 = 7
    //    1   1       1      8  6 * 1 + 2 = 8
    //    1   2       2      9  6 * 1 + 3 = 9
    //    1   3       3     10  6 * 1 + 4 = 10
    //    1   4       8     11  6 * 1 + 5 = 11
    //    1   5       9     12  6 * 1 + 5 = 11
    for (i, sample) in buf.iter().enumerate() {
        // Ignore the first sample in the column (as it's data from the previous column)
        if i == 0 {
            continue;
        }

        // Handle multiple samples from the same buffer
        let channel = (sample & 0xF000) >> 12;
        let sample = sample & 0x0FFF;

        // Remap channels to rows
        let row = match channel {
            0..=3 => channel,
            8 => 4,
            9 => 5,
            _ => continue,
        } as usize;

        let index = row * CSIZE + strobe;
        match matrix.record::<ADC_SAMPLES>(index, sample) {
            Ok(val) => {
                // If data bucket has accumulated enough samples, pass to the next stage
                if let Some(sense) = val {
                    // Store data for manufacturing test results
                    if collect_manu_test {
                        manu_test_data
                            .extend_from_slice(&sense.raw.to_le_bytes())
                            .unwrap();
                    }

                    for event in sense.trigger_event(switch_remap[index] as usize, false) {
                        let _hidio_event = HidIoEvent::TriggerEvent(event);

                        // Enqueue KLL trigger event
                        let ret = layer_state.process_trigger::<MAX_LAYER_LOOKUP_SIZE>(event);
                        assert!(ret.is_ok(), "Failed to enqueue: {:?} - {:?}", event, ret);

                        /* TODO - Logs too noisy currently
                        // Enqueue HID-IO trigger event
                            if let Err(err) = hidio_intf.process_event(hidio_event)
                            {
                                defmt::error!(
                                    "Hidio TriggerEvent Error: {:?}",
                                    err
                                );
                            }
                        */
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
        // When the manufacturing test buffer is full, send it and clear the buffer
        if collect_manu_test && manu_test_data.len() + 2 + RSIZE > manu_test_data.capacity() {
            // If manufacturing test is enabled, send accumulated data
            if hidio_intf.interface().manufacturing_config.hall_level_check
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
        }
        // Increment strobe cycle
        if strobe == 0 {
            *strobe_cycle = strobe_cycle.wrapping_add(1);
        }
    }

    // Prepare next DMA read, but don't start it yet
    adc_pdc.replace(adc.read_paused(buf));

    strobe
}
