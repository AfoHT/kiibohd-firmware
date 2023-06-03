// Copyright 2021-2023 Jacob Alexander
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use crate::constants::*;
use gemini::{kll, Pins};
use kiibohd_atsam4s::{
    self,
    constants::*,
    hal::{
        clock::{Enabled, MainClock, SlowClock, Tc0Clock, Tc1Clock},
        gpio::*,
        pac::TC0,
        prelude::*,
        timer::TimerCounterChannel,
        udp::{usb_device::bus::UsbBusAllocator, usb_device::device::UsbDeviceState, UdpBus},
        watchdog::Watchdog,
        ToggleableOutputPin,
    },
    heapless::{
        spsc::{Consumer, Producer, Queue},
        String,
    },
    kiibohd_usb, LayerState, UsbState,
};
use rtic_monotonics::systick::*;

mod constants;

// ----- RTIC -----

// RTIC requires that unused interrupts are declared in an extern block when
// using software tasks; these free interrupts will be used to dispatch the
// software tasks.
#[rtic::app(device = kiibohd_atsam4s::hal::pac, peripherals = true, dispatchers = [UART1, USART0, USART1, SSC, PWM, ACC, ADC, SPI])]
mod app {
    use super::*;

    // ----- Types -----

    type LayerLookup = kiibohd_atsam4s::kll_core::layout::LayerLookup<'static, LAYOUT_SIZE>;
    type Matrix = kiibohd_atsam4s::keyscanning::KeyMatrix<CSIZE, RSIZE, MSIZE, SCAN_PERIOD_US>;

    // ----- Structs -----

    //
    // Shared resources used by tasks/interrupts
    //
    #[shared]
    struct Shared {
        hidio_intf: kiibohd_atsam4s::HidioCommandInterface,
        layer_state: kiibohd_atsam4s::LayerState,
        matrix: Matrix,
        usb_dev: kiibohd_atsam4s::UsbDevice,
        usb_hid: kiibohd_atsam4s::HidInterface,
    }

    //
    // Local resources, static mut variables, no locking necessary
    // (e.g. can be initialized in init and used in 1 other task function)
    //
    #[local]
    struct Local {
        ctrl_producer: Producer<'static, kiibohd_usb::CtrlState, CTRL_QUEUE_SIZE>,
        debug_led: Pb0<Output<PushPull>>,
        kbd_led_consumer: Consumer<'static, kiibohd_usb::LedState, KBD_LED_QUEUE_SIZE>,
        kbd_producer: Producer<'static, kiibohd_usb::KeyState, KBD_QUEUE_SIZE>,
        mouse_producer: Producer<'static, kiibohd_usb::MouseState, MOUSE_QUEUE_SIZE>,
        rtt: kiibohd_atsam4s::RealTimeTimer,
        tcc0: TimerCounterChannel<TC0, Tc0Clock<Enabled>, 0, TCC0_FREQ>,
        tcc1: TimerCounterChannel<TC0, Tc1Clock<Enabled>, 1, TCC1_FREQ>,
        usb_state: UsbDeviceState,
        //usb_state_consumer: Consumer<'static, kiibohd_atsam4s::UsbState, USB_STATE_QUEUE_SIZE>,
        usb_state_producer: Producer<'static, kiibohd_atsam4s::UsbState, USB_STATE_QUEUE_SIZE>,
        wdt: Watchdog,
    }

    //
    // Initialization
    //
    #[init(
        local = [
            ctrl_queue: Queue<kiibohd_usb::CtrlState, CTRL_QUEUE_SIZE> = Queue::new(),
            kbd_queue: Queue<kiibohd_usb::KeyState, KBD_QUEUE_SIZE> = Queue::new(),
            kbd_led_queue: Queue<kiibohd_usb::LedState, KBD_LED_QUEUE_SIZE> = Queue::new(),
            mouse_queue: Queue<kiibohd_usb::MouseState, MOUSE_QUEUE_SIZE> = Queue::new(),
            usb_state_queue: Queue<UsbState, USB_STATE_QUEUE_SIZE> = Queue::new(),
            serial_number: String<126> = String::new(),
            usb_bus: Option<UsbBusAllocator<UdpBus>> = None,
    ])]
    fn init(cx: init::Context) -> (Shared, Local) {
        let (wdt, clocks, chip, mut tc0_chs, rtt, gpio_ports) = kiibohd_atsam4s::initial_init(
            cx.device.CHIPID,
            cx.device.EFC0,
            cx.device.PIOA,
            cx.device.PIOB,
            cx.device.PMC,
            cx.device.RTT,
            &cx.device.SUPC,
            cx.device.TC0,
            cx.device.WDT,
            MainClock::Crystal12Mhz,
            SlowClock::RcOscillator32Khz,
            cx.local.serial_number,
            VERGEN_GIT_COMMIT_COUNT.parse().unwrap(),
        );

        // Setup pins
        let pins = Pins::new(gpio_ports, &cx.device.MATRIX);

        // Setup Keyscanning Matrix
        let matrix = kiibohd_atsam4s::keyscanning::init::<CSIZE, RSIZE, MSIZE, SCAN_PERIOD_US>(
            [
                pins.strobe1.downgrade(),
                pins.strobe2.downgrade(),
                pins.strobe3.downgrade(),
                pins.strobe4.downgrade(),
                pins.strobe5.downgrade(),
                pins.strobe6.downgrade(),
                pins.strobe7.downgrade(),
                pins.strobe8.downgrade(),
                pins.strobe9.downgrade(),
                pins.strobe10.downgrade(),
                pins.strobe11.downgrade(),
                pins.strobe12.downgrade(),
                pins.strobe13.downgrade(),
                pins.strobe14.downgrade(),
                pins.strobe15.downgrade(),
                pins.strobe16.downgrade(),
                pins.strobe17.downgrade(),
            ],
            [
                pins.sense1.downgrade(),
                pins.sense2.downgrade(),
                pins.sense3.downgrade(),
                pins.sense4.downgrade(),
                pins.sense5.downgrade(),
                pins.sense6.downgrade(),
            ],
            &mut tc0_chs,
        );

        // Setup kll-core
        let loop_condition_lookup: &[u32] = &[0]; // TODO: Use KLL Compiler

        // Load datastructures into kll-core
        let layer_lookup = LayerLookup::new(
            kll::LAYER_LOOKUP,
            kll::TRIGGER_GUIDES,
            kll::RESULT_GUIDES,
            kll::TRIGGER_RESULT_MAPPING,
            loop_condition_lookup,
        );

        // Initialize LayerState for kll-core
        let layer_state = LayerState::new(layer_lookup, 0);

        // Setup USB + HID-IO interface
        let (usb_state_producer, _usb_state_consumer) = cx.local.usb_state_queue.split();
        let usb_state = UsbDeviceState::Default;
        let (
            usb_dev,
            usb_hid,
            hidio_intf,
            ctrl_producer,
            kbd_led_consumer,
            kbd_producer,
            mouse_producer,
        ) = kiibohd_atsam4s::usb_init(
            &chip,
            cx.local.ctrl_queue,
            cx.local.kbd_led_queue,
            cx.local.kbd_queue,
            VERGEN_GIT_COMMIT_COUNT.parse().unwrap(),
            VERGEN_GIT_SEMVER,
            cx.local.mouse_queue,
            cx.local.serial_number,
            cx.device.UDP,
            clocks.peripheral_clocks.udp,
            pins.udp_ddm,
            pins.udp_ddp,
            cx.local.usb_bus,
        );

        // LED Frame Timer
        let mut tcc1 = tc0_chs.ch1;
        tcc1.clock_input(TCC1_DIV);
        tcc1.start(17_u32.millis()); // 17 ms -> ~60 fps (16.6667 ms)
        defmt::trace!("TCC1 started - LED Frame Scheduling");
        tcc1.enable_interrupt();

        // Initialize tickless monotonic timer
        let mono_token = rtic_monotonics::create_systick_token!();
        Systick::start(cx.core.SYST, MCU_FREQ, mono_token);
        defmt::trace!("Systick (Monotonic) started");

        // Manufacturing test data buffer
        (
            Shared {
                hidio_intf,
                layer_state,
                matrix,
                usb_dev,
                usb_hid,
            },
            Local {
                ctrl_producer,
                debug_led: pins.debug_led,
                kbd_led_consumer,
                kbd_producer,
                mouse_producer,
                rtt,
                tcc0: tc0_chs.ch0,
                tcc1,
                usb_state,
                //usb_state_consumer,
                usb_state_producer,
                wdt,
            },
        )
    }

    /// Timer task (TC0)
    /// - Keyscanning Task (Uses tcc0)
    ///   High-priority scheduled tasks as consistency is more important than speed for scanning
    ///   key states
    ///   Scans one strobe at a time
    #[task(priority = 13, binds = TC0, local = [
        tcc0,
    ], shared = [
        hidio_intf,
        layer_state,
        matrix,
    ])]
    fn tc0(cx: tc0::Context) {
        let hidio_intf = cx.shared.hidio_intf;
        let layer_state = cx.shared.layer_state;
        let matrix = cx.shared.matrix;

        // Check for keyscanning interrupt (tcc0)
        (hidio_intf, layer_state, matrix).lock(|hidio_intf, layer_state, matrix| {
            let process_macros = kiibohd_atsam4s::keyscanning::tc0_irq::<
                CSIZE,
                RSIZE,
                MSIZE,
                SCAN_PERIOD_US,
            >(
                hidio_intf, layer_state, matrix, SWITCH_REMAP, cx.local.tcc0
            );

            // If a full matrix scanning cycle has finished, process macros
            if process_macros && macro_process::spawn().is_err() {
                defmt::warn!("Could not schedule macro_process");
            }
        });
    }

    /// Timer task (TC1)
    /// - LED frame scheduling (Uses tcc1)
    ///   Schedules a lower priority task which is skipped if the previous frame is still
    ///   processing
    #[task(priority = 13, binds = TC1, local = [
        tcc1,
    ], shared = [])]
    fn tc1(cx: tc1::Context) {
        // Check for LED frame scheduling interrupt
        if cx.local.tcc1.clear_interrupt_flags() {
            // Attempt to schedule LED frame
            if led_frame_process::spawn().is_err() {
                defmt::warn!("Unable to schedule frame...FPS unstable");
            }
        }
    }

    /// Activity tick
    /// Used visually determine MCU status
    #[task(priority = 1, binds = RTT, local = [
        debug_led,
        rtt,
        wdt,
    ], shared = [])]
    fn rtt(cx: rtt::Context) {
        cx.local.rtt.clear_interrupt_flags();

        // Feed watchdog
        cx.local.wdt.feed();

        // Blink debug led
        // TODO: Remove (or use feature flag)
        cx.local.debug_led.toggle().ok();
    }

    /// LED Frame Processing Task
    /// Handles each LED frame, triggered at a constant rate.
    /// Frames are skipped if the previous frame is still processing.
    #[task(priority = 8, shared = [
        hidio_intf,
    ])]
    async fn led_frame_process(mut cx: led_frame_process::Context) {
        cx.shared.hidio_intf.lock(|_hidio_intf| {
            // TODO
        });

        // Handle processing of "next" frame
        // If this takes too long, the next frame update won't be scheduled (i.e. it'll be
        // skipped).
        // TODO - KLL Pixelmap
        // TODO - HIDIO frame updates
    }

    /// Macro Processing Task
    /// Handles incoming key scan triggers and turns them into results (actions and hid events)
    /// Has a lower priority than keyscanning to schedule around it.
    #[task(priority = 10, local = [
        ctrl_producer,
        kbd_led_consumer,
        kbd_producer,
        mouse_producer,
    ], shared = [
        hidio_intf,
        layer_state,
        matrix,
    ])]
    async fn macro_process(mut cx: macro_process::Context) {
        (cx.shared.layer_state, cx.shared.matrix).lock(|layer_state, matrix| {
            // Query HID LED Events
            cx.shared.hidio_intf.lock(|hidio_intf| {
                kiibohd_atsam4s::macro_process_led_events_task(
                    cx.local.kbd_led_consumer,
                    hidio_intf,
                    layer_state,
                );
            });

            // Process macros
            kiibohd_atsam4s::macro_process_task::<CSIZE, MSIZE, Matrix>(
                cx.local.ctrl_producer,
                cx.local.kbd_producer,
                cx.local.mouse_producer,
                layer_state,
                matrix,
            );
        });

        // Schedule USB processing
        if usb_process::spawn().is_err() {
            defmt::warn!("Could not schedule usb_process");
        }
    }

    /// USB Outgoing Events Task
    /// Sends outgoing USB HID events generated by the macro_process task
    /// Has a lower priority than keyscanning to schedule around it.
    #[task(priority = 11, local = [
        usb_state,
        usb_state_producer,
    ], shared = [
        usb_dev,
        usb_hid,
    ])]
    async fn usb_process(cx: usb_process::Context) {
        let usb_dev = cx.shared.usb_dev;
        let usb_hid = cx.shared.usb_hid;
        (usb_hid, usb_dev).lock(|usb_hid, usb_dev| {
            kiibohd_atsam4s::usb_process_task(
                usb_dev,
                usb_hid,
                cx.local.usb_state,
                cx.local.usb_state_producer,
            );
        });
    }

    /// ISSI I2C0 Interrupt
    #[task(priority = 12, binds = TWI0)]
    fn twi0(_: twi0::Context) {
        //unsafe { TWI0_Handler() };
    }

    /// ISSI I2C1 Interrupt
    #[task(priority = 12, binds = TWI1)]
    fn twi1(_: twi1::Context) {}

    /// USB Device Interupt
    #[task(priority = 14, binds = UDP, shared = [
        hidio_intf,
        usb_dev,
        usb_hid,
    ])]
    fn udp(cx: udp::Context) {
        let usb_dev = cx.shared.usb_dev;
        let usb_hid = cx.shared.usb_hid;
        let hidio_intf = cx.shared.hidio_intf;

        // Poll USB endpoints
        (usb_dev, usb_hid, hidio_intf).lock(|usb_dev, usb_hid, hidio_intf| {
            kiibohd_atsam4s::udp_irq(usb_dev, usb_hid, hidio_intf);
        });
    }
}
