// Copyright 2021-2022 Jacob Alexander
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

#![no_std]

use kiibohd_atsam4s::hal::{define_pin_map, gpio::*, pac::MATRIX};
use paste::paste;

/// [AUTO GENERATED]
pub mod kll {
    //#![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/generated_kll.rs"));
}

define_pin_map! {
    struct Pins,

    // Debug LED (LED4)
    pin debug_led = a15<Output<PushPull>, into_push_pull_output>,

    // Sense
    pin sense1 = a17<ExFn, into_extra_function>,
    pin sense2 = a18<ExFn, into_extra_function>,
    pin sense3 = a19<ExFn, into_extra_function>,
    pin sense4 = a20<ExFn, into_extra_function>,
    pin sense5 = a21<ExFn, into_extra_function>,
    pin sense6 = a22<ExFn, into_extra_function>,

    // Strobe
    pin strobe1 = b0<Output<PushPull>, into_push_pull_output>,
    pin strobe2 = b1<Output<PushPull>, into_push_pull_output>,
    pin strobe3 = b2<Output<PushPull>, into_push_pull_output>,
    pin strobe4 = b3<Output<PushPull>, into_push_pull_output>,
    pin strobe5 = b4<Output<PushPull>, into_push_pull_output>,
    pin strobe6 = b5<Output<PushPull>, into_push_pull_output>,
    pin strobe7 = b14<Output<PushPull>, into_push_pull_output>,
    pin strobe8 = a0<Output<PushPull>, into_push_pull_output>,
    pin strobe9 = a1<Output<PushPull>, into_push_pull_output>,
    pin strobe10 = a2<Output<PushPull>, into_push_pull_output>,
    pin strobe11 = a5<Output<PushPull>, into_push_pull_output>,
    pin strobe12 = a6<Output<PushPull>, into_push_pull_output>,
    pin strobe13 = a7<Output<PushPull>, into_push_pull_output>,
    pin strobe14 = a8<Output<PushPull>, into_push_pull_output>,
    pin strobe15 = a9<Output<PushPull>, into_push_pull_output>,
    pin strobe16 = a10<Output<PushPull>, into_push_pull_output>,
    pin strobe17 = a23<Output<PushPull>, into_push_pull_output>,
    pin strobe18 = a24<Output<PushPull>, into_push_pull_output>,
    pin strobe19 = a25<Output<PushPull>, into_push_pull_output>,
    pin strobe20 = a26<Output<PushPull>, into_push_pull_output>,
    pin strobe21 = a27<Output<PushPull>, into_push_pull_output>,
    pin strobe22 = a28<Output<PushPull>, into_push_pull_output>,

    // Cfg
    pin cfg1 = a29<Input<PullDown>, into_pull_down_input>,
    pin cfg2 = a30<Input<PullDown>, into_pull_down_input>,

    // ISSI
    //pin issi_sdb = a15<Output<PushPull>, into_push_pull_output>, // Already owned by debug_led
    pin issi0_cs = a11<PfA, into_peripheral_function_a>,
    pin issi1_cs = a31<PfA, into_peripheral_function_a>,

    // PWM
    pin solenoid = a16<Output<PushPull>, into_push_pull_output>,

    // DAC
    pin dac = b13<ExFn, into_extra_function>,

    // SPI
    pin spi_miso = a12<PfA, into_peripheral_function_a>,
    pin spi_mosi = a13<PfA, into_peripheral_function_a>,
    pin spi_sck = a14<PfA, into_peripheral_function_a>,

    // Serial Console (UART1)
    // NOTE: Disabled by default
    // TODO (HaaTa): Add debug feature flag to enable uart
    //pin uart_rx = a9<PfA, into_peripheral_function_a>,
    //pin uart_tx = a10<PfA, into_peripheral_function_a>,

    // USB (UDP)
    pin udp_ddm = b10<SysFn, into_system_function>,
    pin udp_ddp = b11<SysFn, into_system_function>,
}
