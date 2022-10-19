// Copyright 2021-2022 Jacob Alexander
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::constants::*;
use crate::*;
pub use is31fl3743b::Is31fl3743bAtsam4Dma;

use core::convert::Infallible;
use fugit::{HertzU32 as Hertz, RateExtU32};
use hal::{
    pdc::{ReadWriteDmaLen, RxTxDma, Transfer, W},
    spi::{SpiMaster, SpiPayload, Variable},
    OutputPin,
};

// ----- Types -----

pub type SpiTransferRxTx = Transfer<
    W,
    (
        &'static mut [u32; SPI_RX_BUF_SIZE],
        &'static mut [u32; SPI_TX_BUF_SIZE],
    ),
    RxTxDma<SpiPayload<Variable, u32>>,
>;
pub type SpiParkedDma = (
    SpiMaster<u32>,
    &'static mut [u32; SPI_RX_BUF_SIZE],
    &'static mut [u32; SPI_TX_BUF_SIZE],
);

// ----- Initialization Functions -----

/// Initializes is31fl3743b LED driver
#[allow(clippy::too_many_arguments)]
pub fn init(
    debug_led: &mut dyn OutputPin<Error = Infallible>,
    issi_default_brightness: u8,
    issi_default_enable: bool,
    spi: hal::pac::SPI,
    spi_clock: hal::clock::SpiClock<Enabled>,
    spi_miso: Pa12<PfA>,
    spi_mosi: Pa13<PfA>,
    spi_rx_buf: &'static mut [u32; SPI_RX_BUF_SIZE],
    spi_sck: Pa14<PfA>,
    spi_tx_buf: &'static mut [u32; SPI_TX_BUF_SIZE],
    tc0_chs: &mut TimerCounterChannels,
) -> (
    SpiTransferRxTx,
    Is31fl3743bAtsam4Dma<ISSI_DRIVER_CHIPS, ISSI_DRIVER_QUEUE_SIZE>,
) {
    // Setup SPI for LED Drivers
    defmt::trace!("SPI ISSI Driver initialization");
    let wdrbt = false; // Wait data read before transfer enabled
    let llb = false; // Local loopback
                     // Cycles to delay between consecutive transfers
    let dlybct = 0; // No delay
    let mut spi = SpiMaster::<u32>::new(
        spi,
        spi_clock,
        spi_miso,
        spi_mosi,
        spi_sck,
        hal::spi::PeripheralSelectMode::Variable,
        wdrbt,
        llb,
        dlybct,
    );

    // Setup each CS channel
    let mode = hal::spi::spi::MODE_3;
    let csa = hal::spi::ChipSelectActive::ActiveAfterTransfer;
    let bits = hal::spi::BitWidth::Width8Bit;
    let baud: Hertz = 12_u32.MHz();
    // Cycles to delay from CS to first valid SPCK
    let dlybs = 0; // Half an SPCK clock period
    let cs_settings = hal::spi::ChipSelectSettings::new(mode, csa, bits, baud, dlybs, dlybct);
    for i in 0..ISSI_DRIVER_CHIPS {
        spi.cs_setup(i as u8, cs_settings.clone()).unwrap();
    }
    spi.enable_txbufe_interrupt();

    // Setup SPI with pdc
    let spi = spi.with_pdc_rxtx();

    // Setup ISSI LED Driver
    let mut issi = Is31fl3743bAtsam4Dma::<ISSI_DRIVER_CHIPS, ISSI_DRIVER_QUEUE_SIZE>::new(
        ISSI_DRIVER_CS_LAYOUT,
        issi_default_brightness,
        issi_default_enable,
    );

    // Disable ISSI hardware shutdown
    debug_led.set_high().ok();

    // Start ISSI LED Driver initialization
    issi.reset().unwrap(); // Queue reset DMA transaction
    let (rx_len, tx_len) = issi.tx_function(spi_tx_buf).unwrap();
    let spi_rxtx = spi.read_write_len(spi_rx_buf, rx_len, spi_tx_buf, tx_len);

    // LED Frame Timer
    let tcc1 = &mut tc0_chs.ch1;
    tcc1.clock_input(TCC1_DIV);
    tcc1.start(17_u32.millis()); // 17 ms -> ~60 fps (16.6667 ms)
    defmt::trace!("TCC1 started - LED Frame Scheduling");
    tcc1.enable_interrupt();

    (spi_rxtx, issi)
}

// ----- Software Interrupt Tasks -----

/// LED Frame Processing Task for manufacturing commands
/// Handles any initial processing needed before doing normal frame processing
/// Returns false if no more frame processing should be done on this iteration
/// (regular_processing, spawn led_test)
pub fn led_frame_process_manufacturing_tests_task(
    hidio_intf: &mut HidioCommandInterface,
    issi: &mut Is31fl3743bAtsam4Dma<ISSI_DRIVER_CHIPS, ISSI_DRIVER_QUEUE_SIZE>,
    led_test: &mut LedTest,
) -> (bool, bool) {
    // Look for manufacturing test commands
    // Only check for new tests if one is not currently running
    match *led_test {
        LedTest::Disabled => {
            if hidio_intf.interface().manufacturing_config.led_short_test {
                // Enqueue short test
                issi.short_circuit_detect_setup().unwrap();
                *led_test = LedTest::ShortQuery;
                // Even though AN-107 - OPEN SHORT TEST FUNCTION OF IS31FL3743B says
                // that only 1ms is required, in practice 2ms seems more reliable.
                // spawn led_test
                hidio_intf
                    .mut_interface()
                    .manufacturing_config
                    .led_short_test = false;
                (false, true)
            } else if hidio_intf.interface().manufacturing_config.led_open_test {
                // Enqueue open test
                issi.open_circuit_detect_setup().unwrap();
                *led_test = LedTest::OpenQuery;
                // Even though AN-107 - OPEN SHORT TEST FUNCTION OF IS31FL3743B says
                // that only 1ms is required, in practice 2ms seems more reliable.
                // spawn led_test
                hidio_intf
                    .mut_interface()
                    .manufacturing_config
                    .led_open_test = false;
                (false, true)
            } else {
                (true, false)
            }
        }
        LedTest::Reset => {
            // Reset LED state
            // The PWM and Scaling registers are reset, but we have a full copy
            // in memory on the MCU so it will be the previous state.
            issi.reset().unwrap();
            *led_test = LedTest::Disabled;
            (false, false)
        }
        _ => (false, false),
    }
}

/// LED Frame Processing Task
/// Handles each LED frame, triggered at a constant rate.
/// Frames are skipped if the previous frame is still processing.
pub fn led_frame_process_is31fl3743b_dma_task(
    hidio_intf: &mut HidioCommandInterface,
    issi: &mut Is31fl3743bAtsam4Dma<ISSI_DRIVER_CHIPS, ISSI_DRIVER_QUEUE_SIZE>,
    spi_periph: &mut Option<SpiParkedDma>,
    spi_rxtx: &mut Option<SpiTransferRxTx>,
    regular_processing: bool,
) {
    // Process incoming Pixel/LED Buffers
    if regular_processing {
        let control = hidio_intf.interface().led_control.control;

        // Determine if HID-IO processing is enabled
        if control != h0021::args::Control::Disable {
            // Check for a reset, otherwise process frame
            if hidio_intf.interface().led_control.hard_reset
                || hidio_intf.interface().led_control.soft_reset
            {
                hidio_intf.mut_interface().led_control.soft_reset = false;
                hidio_intf.mut_interface().led_control.hard_reset = false;
                issi.reset().unwrap(); // Queue reset DMA transaction
                issi.scaling().unwrap(); // Queue scaling default
                issi.pwm().unwrap(); // Queue pwm default
            } else if (control == h0021::args::Control::EnablePause
                && hidio_intf.interface().led_control.next_frame)
                || control == h0021::args::Control::EnableStart
            {
                // Process frame
                hidio_intf.mut_interface().led_control.next_frame = false;

                // Copy data to frame buffer
                for (i, chip) in issi.pwm_page_buf().iter_mut().enumerate() {
                    let start = i * ISSI_DRIVER_CHANNELS;
                    let end = (i + 1) * ISSI_DRIVER_CHANNELS;
                    chip.copy_from_slice(&hidio_intf.interface().led_buffer[start..end]);
                }
                issi.pwm().unwrap(); // Queue pwm default
            }
        }
    }

    // Enable SPI DMA to update frame
    // We only need to re-enable DMA if the queue was previously empty and "parked"
    if let Some((mut spi, rx_buf, tx_buf)) = spi_periph.take() {
        spi.enable_txbufe_interrupt();
        let spi = spi.with_pdc_rxtx();
        // Look for issi event queue
        if let Ok((rx_len, tx_len)) = issi.tx_function(tx_buf) {
            spi_rxtx.replace(spi.read_write_len(rx_buf, rx_len, tx_buf, tx_len));
        } else {
            // Nothing to do (repark dma)
            let mut spi = spi.revert();
            spi.disable_txbufe_interrupt();
            spi_periph.replace((spi, rx_buf, tx_buf));
        }
    }
}

/// LED Test Results
/// Asynchronous task to handle LED test results (both short and open).
/// This task is schedule at least 750 us after the test is started.
///
/// Returns true if led_test and led_frame_process should be scheduled
pub fn led_test_task(
    hidio_intf: &mut HidioCommandInterface,
    issi: &mut Is31fl3743bAtsam4Dma<ISSI_DRIVER_CHIPS, ISSI_DRIVER_QUEUE_SIZE>,
    led_test: &mut LedTest,
) -> bool {
    // Check for test results
    match *led_test {
        LedTest::ShortQuery => {
            // Schedule read of the short test results
            issi.short_circuit_detect_read().unwrap();
            *led_test = LedTest::ShortReady;
            // NOTE: This should be quick, but we don't want to poll

            // Spawn led_test and led_frame_process
            true
        }
        LedTest::ShortReady => {
            // Read short results
            let short_results = issi.short_circuit_raw().unwrap();

            // 1 byte id, 1 byte length, 32 bytes of data, 1 byte id, ...
            // Buffer size defined by kiibohd_hidio
            let mut data: heapless::Vec<u8, { kiibohd_hid_io::MESSAGE_LEN - 4 }> =
                heapless::Vec::new();
            data.push(0).unwrap(); // Id
            data.push(32).unwrap(); // Length
            data.extend_from_slice(&short_results[0]).unwrap(); // Data
            data.push(1).unwrap(); // Id
            data.push(32).unwrap(); // Length
            data.extend_from_slice(&short_results[1]).unwrap(); // Data
            hidio_intf
                .h0051_manufacturingres(h0051::Cmd {
                    command: h0051::Command::LedTestSequence,
                    argument: h0051::Argument {
                        led_test_sequence: h0051::args::LedTestSequence::LedShortTest,
                    },
                    data,
                })
                .unwrap();

            *led_test = LedTest::Reset;

            false
        }
        LedTest::OpenQuery => {
            // Schedule read of the short test results
            issi.open_circuit_detect_read().unwrap();
            *led_test = LedTest::OpenReady;
            // NOTE: This should be quick, but we don't want to poll

            // Spawn led_test and led_frame_process
            true
        }
        LedTest::OpenReady => {
            // Read short results
            let open_results = issi.open_circuit_raw().unwrap();

            // 1 byte id, 1 byte length, 32 bytes of data, 1 byte id, ...
            // Buffer size defined by kiibohd_hidio
            let mut data: heapless::Vec<u8, { kiibohd_hid_io::MESSAGE_LEN - 4 }> =
                heapless::Vec::new();
            data.push(0).unwrap(); // Id
            data.push(32).unwrap(); // Length
            data.extend_from_slice(&open_results[0]).unwrap(); // Data
            data.push(1).unwrap(); // Id
            data.push(32).unwrap(); // Length
            data.extend_from_slice(&open_results[1]).unwrap(); // Data
            hidio_intf
                .h0051_manufacturingres(h0051::Cmd {
                    command: h0051::Command::LedTestSequence,
                    argument: h0051::Argument {
                        led_test_sequence: h0051::args::LedTestSequence::LedOpenCircuitTest,
                    },
                    data,
                })
                .unwrap();

            *led_test = LedTest::Reset;

            false
        }
        _ => false,
    }
}

// ----- IRQ Functions -----

/// SPI Interrupt
pub fn spi_irq(
    issi: &mut Is31fl3743bAtsam4Dma<ISSI_DRIVER_CHIPS, ISSI_DRIVER_QUEUE_SIZE>,
    spi_periph: &mut Option<SpiParkedDma>,
    spi_rxtx: &mut Option<SpiTransferRxTx>,
) {
    // Retrieve DMA buffer
    if let Some(spi_buf) = spi_rxtx.take() {
        let ((rx_buf, tx_buf), spi) = spi_buf.wait();

        // Process Rx buffer if applicable
        issi.rx_function(rx_buf).unwrap();

        // Prepare the next DMA transaction
        if let Ok((rx_len, tx_len)) = issi.tx_function(tx_buf) {
            spi_rxtx.replace(spi.read_write_len(rx_buf, rx_len, tx_buf, tx_len));
        } else {
            // Disable PDC
            let mut spi = spi.revert();
            spi.disable_txbufe_interrupt();

            // No more transactions ready, park spi peripheral and buffers
            spi_periph.replace((spi, rx_buf, tx_buf));
        }
    }
}
