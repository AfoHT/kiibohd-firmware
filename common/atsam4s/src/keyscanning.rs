// Copyright 2021-2022 Jacob Alexander
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::constants::*;
use crate::*;
use core::convert::Infallible;
use hal::timer::TimerCounterChannel;

// ----- Types -----

pub type KeyMatrix<
    const CSIZE: usize,
    const RSIZE: usize,
    const MSIZE: usize,
    const SCAN_PERIOD_US: u32,
> = kiibohd_keyscanning::Matrix<
    PioX<Output<PushPull>>,
    PioX<Input<PullDown>>,
    CSIZE,
    RSIZE,
    MSIZE,
    SCAN_PERIOD_US,
    DEBOUNCE_US,
    IDLE_MS,
>;

// ----- Initialization Functions -----

pub fn init<
    const CSIZE: usize,
    const RSIZE: usize,
    const MSIZE: usize,
    const SCAN_PERIOD_US: u32,
>(
    cols: [PioX<Output<PushPull>>; CSIZE],
    rows: [PioX<Input<PullDown>>; RSIZE],
    tc0_chs: &mut TimerCounterChannels,
) -> KeyMatrix<CSIZE, RSIZE, MSIZE, SCAN_PERIOD_US> {
    // Setup Keyscanning Matrix
    defmt::trace!("Keyscanning Matrix initialization");
    let mut matrix = KeyMatrix::new(cols, rows).unwrap();
    matrix.next_strobe().unwrap(); // Initial strobe

    // Keyscanning Timer
    let tcc0 = &mut tc0_chs.ch0;
    tcc0.clock_input(TCC0_DIV);
    tcc0.start((SCAN_PERIOD_US * 1000).nanos());
    defmt::trace!("TCC0 started - Keyscanning");
    tcc0.enable_interrupt();

    matrix
}

// ----- Software Interrupt Tasks -----

// ----- IRQ Functions -----

/// Timer task (TC0)
/// - Keyscanning Task (Uses tcc0)
///   High-priority scheduled tasks as consistency is more important than speed for scanning
///   key states
///   Scans one strobe at a time
/// Returns true after all strobes have been scanned and macros should be processed
pub fn tc0_irq<
    const CSIZE: usize,
    const RSIZE: usize,
    const MSIZE: usize,
    const SCAN_PERIOD_US: u32,
>(
    hidio_intf: &mut HidioCommandInterface,
    layer_state: &mut LayerState,
    matrix: &mut KeyMatrix<CSIZE, RSIZE, MSIZE, SCAN_PERIOD_US>,
    switch_remap: &[u8],
    tcc0: &mut TimerCounterChannel<TC0, Tc0Clock<Enabled>, 0, TCC0_FREQ>,
) -> bool {
    // Check for keyscanning interrupt (tcc0)
    if tcc0.clear_interrupt_flags() {
        // Scan one strobe (strobes have already been enabled and allowed to settle)
        if let Ok((reading, strobe)) = matrix.sense::<Infallible>() {
            for (i, entry) in reading.iter().enumerate() {
                for event in entry.trigger_event(switch_remap[strobe * RSIZE + i] as usize, true) {
                    let hidio_event = HidIoEvent::TriggerEvent(event);

                    // Enqueue KLL trigger event
                    let ret = layer_state.process_trigger::<MAX_LAYER_LOOKUP_SIZE>(event);
                    debug_assert!(ret.is_ok(), "Failed to enqueue: {:?} - {:?}", event, ret);

                    // Enqueue HID-IO trigger event
                    if let Err(err) = hidio_intf.process_event(hidio_event) {
                        defmt::error!("Hidio TriggerEvent Error: {:?}", err);
                    }
                }
            }
        }

        // Strobe next column
        return matrix.next_strobe::<Infallible>().unwrap() == 0;
    }

    false
}
