// Copyright 2021-2022 Jacob Alexander
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

#![no_std]

pub mod constants;
mod hidio;

#[cfg(feature = "hall-effect")]
pub mod hall_effect;

#[cfg(feature = "issi-spi")]
pub mod issi_spi;

#[cfg(feature = "keyscanning")]
pub mod keyscanning;

pub use atsam4_hal as hal;
pub use heapless;
pub use kiibohd_hid_io;
pub use kiibohd_usb;
pub use kll_core;

use defmt_rtt as _;
use panic_probe as _;

use crate::constants::*;
use crate::hidio::*;
use core::fmt::Write;
use cortex_m_rt::exception;
use dwt_systick_monotonic::*;
use hal::{
    chipid::ChipId,
    clock::{ClockController, Disabled, Enabled, Tc0Clock, Tc1Clock, Tc2Clock},
    efc::Efc,
    gpio::*,
    pac::TC0,
    prelude::*,
    timer::TimerCounter,
    udp::{
        usb_device,
        usb_device::{
            bus::UsbBusAllocator,
            device::{UsbDeviceBuilder, UsbDeviceState, UsbVidPid},
        },
        UdpBus,
    },
    watchdog::Watchdog,
};
use heapless::{
    spsc::{Consumer, Producer, Queue},
    String,
};
use kiibohd_hid_io::*;
use kiibohd_usb::HidCountryCode;

// ----- Types -----

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum LedTest {
    /// No active test
    Disabled,
    /// Reset led controller (after reading test data, before setting disabled).
    Reset,
    /// Active test, need to query results from controller (next state is OpenReady)
    OpenQuery,
    /// Test finished, can read results directly (next state is Reset)
    OpenReady,
    /// Active test, need to query results from controller (next state is ShortReady)
    ShortQuery,
    /// Test finished, can read results directly (next state is Reset)
    ShortReady,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum UsbState {
    /// Transitioned to a suspended state
    Suspend,
    /// Transitioned to an active state
    Resume,
}

pub type HidInterface = kiibohd_usb::HidInterface<
    'static,
    UdpBus,
    KBD_QUEUE_SIZE,
    KBD_LED_QUEUE_SIZE,
    MOUSE_QUEUE_SIZE,
    CTRL_QUEUE_SIZE,
>;
pub type HidioCommandInterface = CommandInterface<
    HidioInterface<MESSAGE_LEN>,
    TX_BUF,
    RX_BUF,
    BUF_CHUNK,
    MESSAGE_LEN,
    SERIALIZATION_LEN,
    ID_LEN,
>;
pub type LayerState = kll_core::layout::LayerState<
    'static,
    LAYOUT_SIZE,
    STATE_SIZE,
    MAX_LAYERS,
    MAX_ACTIVE_LAYERS,
    MAX_ACTIVE_TRIGGERS,
    MAX_LAYER_STACK_CACHE,
    MAX_OFF_STATE_LOOKUP,
>;
pub type RealTimeTimer = hal::rtt::RealTimeTimer<RTT_PRESCALER, false>;
type TimerCounterChannels = hal::timer::TimerCounterChannels<
    TC0,
    Tc0Clock<Enabled>,
    Tc1Clock<Enabled>,
    Tc2Clock<Enabled>,
    TCC0_FREQ,
    TCC1_FREQ,
    TCC2_FREQ,
>;
pub type UsbDevice = usb_device::device::UsbDevice<'static, UdpBus>;

// ----- Structs -----

pub struct IndicatorLeds<const NUM_LEDS: usize> {
    leds: [bool; NUM_LEDS],
}

impl<const NUM_LEDS: usize> IndicatorLeds<NUM_LEDS> {
    pub fn new() -> Self {
        Self {
            leds: [false; NUM_LEDS],
        }
    }

    pub fn set(&mut self, index: usize, state: bool) {
        self.leds[index] = state;
    }

    pub fn get(&self, index: usize) -> bool {
        self.leds[index]
    }
}

impl<const NUM_LEDS: usize> Default for IndicatorLeds<NUM_LEDS> {
    fn default() -> Self {
        Self::new()
    }
}

// ----- Initialization Functions -----

/// Check user signature (up to 512-bytes of data)
/// Currently only used to store the firmware revision number (16-bits)
fn check_user_signature(efc: &mut Efc, revision: u16) -> bool {
    // Read signature, and verify it
    let mut sig: [u32; 1] = [0; 1];
    efc.read_user_signature(&mut sig, 1).unwrap();

    // First check if the user signature is empty
    if sig[0] == 0xFFFFFFFF {
        // Signature is empty, write the firmware revision number
        sig[0] = 0xFFFF0000 | revision as u32;
        efc.write_user_signature(&sig).unwrap();
    } else {
        // Signature is not empty, check if it matches the firmware revision number
        if sig[0] & 0x0000FFFF != revision as u32 {
            defmt::error!(
                "Firmware revision mismatch! Expected: {:x}, Got: {:x}",
                revision,
                sig[0] & 0xFFFF
            );
            return false;
        }
    }

    true
}

/// Early initialization for atsam4s
/// Handles
/// - Chip ID
/// - Clocks
/// - Watchdog
/// - Flash Controller (EFC0)
/// - Serial Number
#[allow(clippy::too_many_arguments)]
pub fn initial_init(
    chipid: hal::pac::CHIPID,
    efc0: hal::pac::EFC0,
    pioa: hal::pac::PIOA,
    piob: hal::pac::PIOB,
    pmc: hal::pac::PMC,
    rtt: hal::pac::RTT,
    supc: &hal::pac::SUPC,
    tc0: hal::pac::TC0,
    wdt: hal::pac::WDT,
    main_clock: hal::clock::MainClock,
    slow_clock: hal::clock::SlowClock,
    serial_number: &mut String<126>,
    revision: u16,
) -> (
    Watchdog,
    ClockController,
    ChipId,
    TimerCounterChannels,
    RealTimeTimer,
    Ports,
) {
    defmt::info!(">>>> Initializing <<<<");

    // Show processor registers
    defmt::trace!("MSP: {:#010x}", cortex_m::register::msp::read());
    defmt::trace!("PSP: {:#010x}", cortex_m::register::psp::read());

    // Determine which chip is running
    let chip = ChipId::new(chipid);
    defmt::info!("MCU: {:?}", chip.model());

    // Setup main and slow clocks
    defmt::trace!("Clock initialization");
    let mut clocks = ClockController::new(pmc, supc, &efc0, main_clock, slow_clock);

    // Setup gpios
    defmt::trace!("GPIO initialization");
    let gpio_ports = Ports::new(
        (pioa, clocks.peripheral_clocks.pio_a.into_enabled_clock()),
        (piob, clocks.peripheral_clocks.pio_b.into_enabled_clock()),
    );

    // Prepare watchdog to be fed
    let mut wdt = Watchdog::new(wdt);
    wdt.feed();
    defmt::trace!("Watchdog first feed");

    // Setup flash controller (needed for unique id)
    let mut efc = Efc::new(efc0, unsafe { &mut FLASH_CONFIG });
    // Retrieve unique id and format it for the USB descriptor
    let uid = efc.read_unique_id().unwrap();
    write!(
        serial_number,
        "{:x}{:x}{:x}{:x}",
        uid[0], uid[1], uid[2], uid[3]
    )
    .unwrap();
    defmt::info!("UID: {}", serial_number);

    // Check user signature
    check_user_signature(&mut efc, revision);
    defmt::info!("Firmware revision: {:x}", revision);

    // Setup main timer
    let tc0 = TimerCounter::new(tc0);
    let tc0_chs: TimerCounterChannels = tc0.split(
        clocks.peripheral_clocks.tc_0.into_enabled_clock(),
        clocks.peripheral_clocks.tc_1.into_enabled_clock(),
        clocks.peripheral_clocks.tc_2.into_enabled_clock(),
    );

    // Setup secondary timer (used for watchdog, activity led and sleep related functionality)
    let mut rtt = RealTimeTimer::new(rtt);
    rtt.start(500_000u32.micros());
    rtt.enable_alarm_interrupt();
    defmt::trace!("RTT Timer started");

    (wdt, clocks, chip, tc0_chs, rtt, gpio_ports)
}

/// Initialize atsam4s UdpBus + kiibohd hid + hid-io
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
pub fn usb_init(
    chip: &ChipId,
    ctrl_queue: &'static mut Queue<kiibohd_usb::CtrlState, CTRL_QUEUE_SIZE>,
    kbd_led_queue: &'static mut Queue<kiibohd_usb::LedState, KBD_LED_QUEUE_SIZE>,
    kbd_queue: &'static mut Queue<kiibohd_usb::KeyState, KBD_QUEUE_SIZE>,
    firmware_commit_count: u16,
    firmware_version: &'static str,
    mouse_queue: &'static mut Queue<kiibohd_usb::MouseState, MOUSE_QUEUE_SIZE>,
    serial_number: &'static String<126>,
    udp: hal::pac::UDP,
    udp_clock: hal::clock::UdpClock<Disabled>,
    udp_ddm: Pb10<SysFn>,
    udp_ddp: Pb11<SysFn>,
    usb_bus: &'static mut Option<UsbBusAllocator<UdpBus>>,
) -> (
    UsbDevice,
    HidInterface,
    HidioCommandInterface,
    Producer<'static, kiibohd_usb::CtrlState, CTRL_QUEUE_SIZE>,
    Consumer<'static, kiibohd_usb::LedState, KBD_LED_QUEUE_SIZE>,
    Producer<'static, kiibohd_usb::KeyState, KBD_QUEUE_SIZE>,
    Producer<'static, kiibohd_usb::MouseState, MOUSE_QUEUE_SIZE>,
) {
    // Setup HID-IO interface
    defmt::trace!("HID-IO Interface initialization");
    let hidio_intf = HidioCommandInterface::new(
        &[
            HidIoCommandId::DirectSet,
            HidIoCommandId::GetInfo,
            HidIoCommandId::ManufacturingTest,
            HidIoCommandId::PixelSetting,
            HidIoCommandId::SupportedIds,
            HidIoCommandId::TestPacket,
        ],
        HidioInterface::<MESSAGE_LEN>::new(chip, Some(serial_number.clone()), firmware_version),
    )
    .unwrap();

    // Setup USB
    defmt::trace!("UDP initialization");
    let (kbd_producer, kbd_consumer) = kbd_queue.split();
    let (kbd_led_producer, kbd_led_consumer) = kbd_led_queue.split();
    let (mouse_producer, mouse_consumer) = mouse_queue.split();
    let (ctrl_producer, ctrl_consumer) = ctrl_queue.split();
    let mut udp_bus = UdpBus::new(udp, udp_clock, udp_ddm, udp_ddp);
    udp_bus.remote_wakeup_enabled(true); // Enable hardware support for remote wakeup
    *usb_bus = Some(UsbBusAllocator::<UdpBus>::new(udp_bus));
    let usb_bus = usb_bus.as_ref().unwrap();
    let usb_hid = HidInterface::new(
        usb_bus,
        HidCountryCode::NotSupported,
        kbd_consumer,
        kbd_led_producer,
        mouse_consumer,
        ctrl_consumer,
    );
    let mut usb_dev = UsbDeviceBuilder::new(usb_bus, UsbVidPid(VID, PID))
        .manufacturer(USB_MANUFACTURER)
        .max_packet_size_0(64)
        .max_power(500)
        .product(USB_PRODUCT)
        .supports_remote_wakeup(true)
        .serial_number(serial_number)
        .device_release(firmware_commit_count)
        .build();

    // TODO This should only really be run when running with a debugger for development
    usb_dev.force_reset().unwrap();

    (
        usb_dev,
        usb_hid,
        hidio_intf,
        ctrl_producer,
        kbd_led_consumer,
        kbd_producer,
        mouse_producer,
    )
}

// ----- Software Interrupt Tasks -----

/// Macro Processing Task
/// Handles incoming key scan triggers and turns them into results (actions and hid events)
/// Has a lower priority than keyscanning to schedule around it.
pub fn macro_process_task<const CSIZE: usize, const MSIZE: usize, MATRIX>(
    ctrl_producer: &mut Producer<'static, kiibohd_usb::CtrlState, CTRL_QUEUE_SIZE>,
    kbd_producer: &mut Producer<'static, kiibohd_usb::KeyState, KBD_QUEUE_SIZE>,
    _mouse_producer: &mut Producer<'static, kiibohd_usb::MouseState, MOUSE_QUEUE_SIZE>,
    layer_state: &mut LayerState,
    matrix: &mut MATRIX,
) where
    MATRIX: kiibohd_keyscanning::KeyScanning<MAX_PER_KEY_EVENTS>,
{
    // Confirm off-state lookups
    layer_state.process_off_state_lookups::<MAX_LAYER_LOOKUP_SIZE, MAX_PER_KEY_EVENTS>(&|index| {
        matrix.generate_events(index)
    });

    // Finalize triggers to generate CapabilityRun events
    for cap_run in layer_state.finalize_triggers::<MAX_LAYER_LOOKUP_SIZE>() {
        match cap_run {
            kll_core::CapabilityRun::NoOp { .. } => {}
            kll_core::CapabilityRun::HidKeyboard { .. }
            | kll_core::CapabilityRun::HidKeyboardState { .. } => {
                debug_assert!(
                    kiibohd_usb::enqueue_keyboard_event(cap_run, kbd_producer).is_ok(),
                    "KBD_QUEUE_SIZE too small"
                );
            }
            kll_core::CapabilityRun::HidProtocol { .. } => {}
            kll_core::CapabilityRun::HidConsumerControl { .. }
            | kll_core::CapabilityRun::HidSystemControl { .. } => {
                debug_assert!(
                    kiibohd_usb::enqueue_ctrl_event(cap_run, ctrl_producer).is_ok(),
                    "CTRL_QUEUE_SIZE too small"
                );
            }
            /*
            kll_core::CapabilityRun::McuFlashMode { .. } => {}
            kll_core::CapabilityRun::HidioOpenUrl { .. }
            | kll_core::CapabilityRun::HidioUnicodeString { .. }
            | kll_core::CapabilityRun::HidioUnicodeState { .. } => {}
            kll_core::CapabilityRun::LayerClear { .. }
            | kll_core::CapabilityRun::LayerRotate { .. }
            | kll_core::CapabilityRun::LayerState { .. } => {}
            */
            _ => {
                panic!("{:?} is unsupported by this keyboard", cap_run);
            }
        }
    }

    // Next time iteration
    layer_state.increment_time();
}

/// Sub-task of macro_process when handling HID LED events
pub fn macro_process_led_events_task(
    kbd_led_consumer: &mut Consumer<'static, kiibohd_usb::LedState, KBD_LED_QUEUE_SIZE>,
    hidio_intf: &mut HidioCommandInterface,
    layer_state: &mut LayerState,
) {
    while let Some(state) = kbd_led_consumer.dequeue() {
        // Convert to a TriggerEvent
        let event = state.trigger_event();
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

/// USB Outgoing Events Task
/// Sends outgoing USB HID events generated by the macro_process task
/// Has a lower priority than keyscanning to schedule around it.
pub fn usb_process_task(
    usb_dev: &mut UsbDevice,
    usb_hid: &mut HidInterface,
    state: &mut UsbDeviceState,
    usb_state_producer: &mut Producer<'static, UsbState, USB_STATE_QUEUE_SIZE>,
) {
    let cur_state = usb_dev.state();
    let prev_state = *state;

    // Suspend -> <Other state> (Resume)
    if prev_state == UsbDeviceState::Suspend && cur_state != UsbDeviceState::Suspend {
        defmt::trace!("USB Resume Event");
        usb_state_producer.enqueue(UsbState::Resume).ok();
    }

    // <Other state> -> Suspend (Suspend)
    if prev_state != UsbDeviceState::Suspend && cur_state == UsbDeviceState::Suspend {
        defmt::trace!("USB Suspend Event");
        usb_state_producer.enqueue(UsbState::Suspend).ok();
    }

    // Update USB events
    if usb_hid.update() {
        match cur_state {
            UsbDeviceState::Suspend => {
                // Issue USB Resume if enabled
                if usb_dev.remote_wakeup_enabled() {
                    usb_dev.bus().remote_wakeup();
                }
            }

            UsbDeviceState::Configured => {
                // Commit USB events
                while usb_hid.push().is_err() {}
            }

            _ => {}
        }
    }

    // Update USB state
    *state = cur_state;
}

// ----- IRQ Functions -----

/// USB Device Interrupt
pub fn udp_irq(
    usb_dev: &mut UsbDevice,
    usb_hid: &mut HidInterface,
    hidio_intf: &mut HidioCommandInterface,
) {
    // Poll USB endpoints
    if usb_dev.poll(&mut usb_hid.interfaces()) {
        // Retrive HID Lock LED events
        usb_hid.pull();

        // Process HID-IO
        usb_hid.pull_hidio(hidio_intf);
    }
    // Attempt to tx any HID-IO packets
    usb_hid.push_hidio(hidio_intf);
}

// ----- Misc Setup Functions -----

#[exception]
unsafe fn HardFault(_ef: &cortex_m_rt::ExceptionFrame) -> ! {
    panic!("HardFault!");
}

defmt::timestamp!("{=u64} us", {
    atsam4_hal::timer::DwtTimer::<{ constants::MCU_FREQ }>::now()
        / ((constants::MCU_FREQ / 1_000_000) as u64)
});

// same panicking *behavior* as `panic-probe` but doesn't print a panic message
// this prevents the panic message being printed *twice* when `defmt::panic` is invoked
#[defmt::panic_handler]
fn panic() -> ! {
    cortex_m::asm::udf()
}

/// Terminates the application and makes `probe-run` exit with exit-code = 0
pub fn exit() -> ! {
    loop {
        cortex_m::asm::bkpt();
    }
}
