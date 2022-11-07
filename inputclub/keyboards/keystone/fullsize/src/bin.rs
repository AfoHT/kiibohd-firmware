// Copyright 2021-2022 Jacob Alexander
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

#![no_std]
#![no_main]

mod constants;

// ----- RTIC -----

// RTIC requires that unused interrupts are declared in an extern block when
// using software tasks; these free interrupts will be used to dispatch the
// software tasks.
#[rtic::app(device = kiibohd_atsam4s::hal::pac, peripherals = true, dispatchers = [UART1, USART0, USART1, SSC, PWM, ACC, TWI0, TWI1])]
mod app {
    use crate::constants::*;
    use dwt_systick_monotonic::*;
    use keystonefs::{kll, Pins};
    use kiibohd_atsam4s::{
        self,
        constants::*,
        hal::{
            clock::{Enabled, MainClock, SlowClock, Tc0Clock, Tc1Clock},
            pac::TC0,
            pdc::ReadDma,
            prelude::*,
            timer::TimerCounterChannel,
            udp::{usb_device::bus::UsbBusAllocator, UdpBus},
            watchdog::Watchdog,
        },
        heapless::{
            self,
            spsc::{Consumer, Producer, Queue},
            String,
        },
        kiibohd_hid_io, kiibohd_usb, LayerState,
    };

    // ----- Types -----

    type LayerLookup = kiibohd_atsam4s::kll_core::layout::LayerLookup<'static, LAYOUT_SIZE>;
    type Matrix = kiibohd_atsam4s::hall_effect::HallMatrix<CSIZE, MSIZE>;

    #[monotonic(binds = SysTick, default = true)]
    type DwtMono = DwtSystick<MCU_FREQ>;

    // ----- Structs -----

    //
    // Shared resources used by tasks/interrupts
    //
    #[shared]
    struct Shared {
        adc: Option<kiibohd_atsam4s::hall_effect::AdcTransfer<ADC_BUF_SIZE>>,
        hidio_intf: kiibohd_atsam4s::HidioCommandInterface,
        issi: kiibohd_atsam4s::issi_spi::Is31fl3743bAtsam4Dma<
            ISSI_DRIVER_CHIPS,
            ISSI_DRIVER_QUEUE_SIZE,
        >,
        layer_state: LayerState,
        led_test: kiibohd_atsam4s::LedTest,
        matrix: Matrix,
        spi: Option<kiibohd_atsam4s::issi_spi::SpiParkedDma>,
        spi_rxtx: Option<kiibohd_atsam4s::issi_spi::SpiTransferRxTx>,
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
        kbd_led_consumer: Consumer<'static, kiibohd_usb::LedState, KBD_LED_QUEUE_SIZE>,
        kbd_producer: Producer<'static, kiibohd_usb::KeyState, KBD_QUEUE_SIZE>,
        manu_test_data: heapless::Vec<u8, { kiibohd_hid_io::MESSAGE_LEN - 4 }>,
        mouse_producer: Producer<'static, kiibohd_usb::MouseState, MOUSE_QUEUE_SIZE>,
        rtt: kiibohd_atsam4s::RealTimeTimer,
        strobe_cycle: u32,
        tcc0: TimerCounterChannel<TC0, Tc0Clock<Enabled>, 0, TCC0_FREQ>,
        tcc1: TimerCounterChannel<TC0, Tc1Clock<Enabled>, 1, TCC1_FREQ>,
        wdt: Watchdog,
    }

    //
    // Initialization
    //
    #[init(
        local = [
            adc_buf: [u16; ADC_BUF_SIZE] = [0; ADC_BUF_SIZE],
            ctrl_queue: Queue<kiibohd_usb::CtrlState, CTRL_QUEUE_SIZE> = Queue::new(),
            kbd_queue: Queue<kiibohd_usb::KeyState, KBD_QUEUE_SIZE> = Queue::new(),
            kbd_led_queue: Queue<kiibohd_usb::LedState, KBD_LED_QUEUE_SIZE> = Queue::new(),
            mouse_queue: Queue<kiibohd_usb::MouseState, MOUSE_QUEUE_SIZE> = Queue::new(),
            serial_number: String<126> = String::new(),
            spi_tx_buf: [u32; SPI_TX_BUF_SIZE] = [0; SPI_TX_BUF_SIZE],
            spi_rx_buf: [u32; SPI_RX_BUF_SIZE] = [0; SPI_RX_BUF_SIZE],
            usb_bus: Option<UsbBusAllocator<UdpBus>> = None,
    ])]
    fn init(mut cx: init::Context) -> (Shared, Local, init::Monotonics) {
        let (wdt, mut clocks, chip, mut tc0_chs, rtt, gpio_ports) = kiibohd_atsam4s::initial_init(
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
        let mut pins = Pins::new(gpio_ports, &cx.device.MATRIX);

        // Setup hall effect matrix
        let (adc, matrix) = kiibohd_atsam4s::hall_effect::init::<CSIZE, MSIZE>(
            cx.device.ADC,
            clocks.peripheral_clocks.adc.into_enabled_clock(),
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
                pins.strobe18.downgrade(),
                pins.strobe19.downgrade(),
                pins.strobe20.downgrade(),
                pins.strobe21.downgrade(),
                pins.strobe22.downgrade(),
            ],
            &mut pins.sense1,
            &mut pins.sense2,
            &mut pins.sense3,
            &mut pins.sense4,
            &mut pins.sense5,
            &mut pins.sense6,
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

        // ISSI + SPI Driver setup
        let issi_default_brightness = 255; // TODO compile-time configuration + flash default
        let issi_default_enable = true; // TODO compile-time configuration + flash default
        let (spi_rxtx, issi) = kiibohd_atsam4s::issi_spi::init(
            &mut pins.debug_led,
            issi_default_brightness,
            issi_default_enable,
            cx.device.SPI,
            clocks.peripheral_clocks.spi.into_enabled_clock(),
            pins.spi_miso,
            pins.spi_mosi,
            cx.local.spi_rx_buf,
            pins.spi_sck,
            cx.local.spi_tx_buf,
            &mut tc0_chs,
        );

        // Setup USB + HID-IO interface
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

        // Initialize tickless monotonic timer
        let mono = DwtSystick::new(&mut cx.core.DCB, cx.core.DWT, cx.core.SYST, MCU_FREQ);
        defmt::trace!("DwtSystick (Monotonic) started");

        // Manufacturing test data buffer
        let manu_test_data = heapless::Vec::new();

        (
            Shared {
                adc: Some(adc.read(cx.local.adc_buf)),
                hidio_intf,
                issi,
                layer_state,
                led_test: kiibohd_atsam4s::LedTest::Disabled,
                matrix,
                spi: None,
                spi_rxtx: Some(spi_rxtx),
                usb_dev,
                usb_hid,
            },
            Local {
                ctrl_producer,
                kbd_led_consumer,
                kbd_producer,
                manu_test_data,
                mouse_producer,
                rtt,
                strobe_cycle: 0,
                tcc0: tc0_chs.ch0,
                tcc1: tc0_chs.ch1,
                wdt,
            },
            init::Monotonics(mono),
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
        adc,
    ])]
    fn tc0(mut cx: tc0::Context) {
        // Check for keyscanning interrupt (tcc0)
        cx.shared.adc.lock(|adc| {
            if cx.local.tcc0.clear_interrupt_flags() {
                // Start next ADC DMA buffer read
                if let Some(adc) = adc {
                    adc.resume();
                }
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
        rtt,
        wdt,
    ], shared = [])]
    fn rtt(cx: rtt::Context) {
        cx.local.rtt.clear_interrupt_flags();

        // Feed watchdog
        cx.local.wdt.feed();
    }

    /// LED Frame Processing Task
    /// Handles each LED frame, triggered at a constant rate.
    /// Frames are skipped if the previous frame is still processing.
    #[task(priority = 8, shared = [
        hidio_intf,
        issi,
        led_test,
        spi,
        spi_rxtx,
    ])]
    fn led_frame_process(cx: led_frame_process::Context) {
        (cx.shared.hidio_intf, cx.shared.issi, cx.shared.led_test).lock(
            |hidio_intf, issi, led_test| {
                // Look for manufacturing test commands
                let (regular_processing, spawn_led_test) =
                    kiibohd_atsam4s::issi_spi::led_frame_process_manufacturing_tests_task(
                        hidio_intf, issi, led_test,
                    );

                // Even though AN-107 - OPEN SHORT TEST FUNCTION OF IS31FL3743B says
                // that only 1ms is required, in practice 2ms seems more reliable.
                if spawn_led_test {
                    led_test::spawn_after(2000_u32.micros()).unwrap();
                }

                // Enable SPI DMA to update frame
                (cx.shared.spi, cx.shared.spi_rxtx).lock(|spi_periph, spi_rxtx| {
                    kiibohd_atsam4s::issi_spi::led_frame_process_is31fl3743b_dma_task(
                        hidio_intf,
                        issi,
                        spi_periph,
                        spi_rxtx,
                        regular_processing,
                    );
                });
            },
        );
    }

    /// LED Test Results
    /// Asynchronous task to handle LED test results (both short and open).
    /// This task is schedule at least 750 us after the test is started.
    #[task(priority = 7, shared = [
        hidio_intf,
        issi,
        led_test,
    ])]
    fn led_test(cx: led_test::Context) {
        // Check for test results
        (cx.shared.hidio_intf, cx.shared.issi, cx.shared.led_test).lock(
            |hidio_intf, issi, led_test| {
                // Check if we need to schedule led_test and led_frame_process
                if kiibohd_atsam4s::issi_spi::led_test_task(hidio_intf, issi, led_test) {
                    led_test::spawn_after(2_u32.millis()).unwrap();
                    led_frame_process::spawn().ok(); // Attempt to schedule frame earlier
                }
            },
        );
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
    fn macro_process(mut cx: macro_process::Context) {
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
            kiibohd_atsam4s::macro_process_task::<CSIZE, MSIZE, Matrix> (
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
    #[task(priority = 11, shared = [
        usb_dev,
        usb_hid,
    ])]
    fn usb_process(cx: usb_process::Context) {
        let usb_dev = cx.shared.usb_dev;
        let usb_hid = cx.shared.usb_hid;
        (usb_hid, usb_dev).lock(|usb_hid, usb_dev| {
            kiibohd_atsam4s::usb_process_task(usb_dev, usb_hid);
        });
    }

    /// ADC Interrupt
    #[task(priority = 14, binds = ADC, local = [
        manu_test_data,
        strobe_cycle,
    ], shared = [
        adc,
        hidio_intf,
        layer_state,
        matrix,
        usb_hid,
    ])]
    fn adc(cx: adc::Context) {
        let adc = cx.shared.adc;
        let hidio_intf = cx.shared.hidio_intf;
        let layer_state = cx.shared.layer_state;
        let manu_test_data = cx.local.manu_test_data;
        let matrix = cx.shared.matrix;

        (adc, hidio_intf, layer_state, matrix).lock(|adc_pdc, hidio_intf, layer_state, matrix| {
            let strobe = kiibohd_atsam4s::hall_effect::adc_irq::<CSIZE, RSIZE, MSIZE, ADC_BUF_SIZE>(
                adc_pdc,
                hidio_intf,
                layer_state,
                manu_test_data,
                matrix,
                cx.local.strobe_cycle,
                SWITCH_REMAP,
            );

            // Process macros after full strobe cycle
            if strobe == 0 && macro_process::spawn().is_err() {
                defmt::warn!("Could not schedule macro_process");
            }
        });
    }

    /// SPI Interrupt
    #[task(priority = 12, binds = SPI, shared = [
        issi,
        spi,
        spi_rxtx,
    ])]
    fn spi(cx: spi::Context) {
        let issi = cx.shared.issi;
        let spi_periph = cx.shared.spi;
        let spi_rxtx = cx.shared.spi_rxtx;

        // Handle SPI DMA transfers
        (issi, spi_periph, spi_rxtx).lock(|issi, spi_periph, spi_rxtx| {
            kiibohd_atsam4s::issi_spi::spi_irq(issi, spi_periph, spi_rxtx);
        });
    }

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
