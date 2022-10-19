// Copyright 2022 Jacob Alexander
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

fn main() {
    // Make sure to rebuild when project.env file env variables change
    let env_vars = [
        "VID",
        "PID",
        "USB_MANUFACTURER",
        "USB_PRODUCT",
        "HIDIO_DEVICE_NAME",
        "HIDIO_DEVICE_VENDOR",
        "HIDIO_FIRMWARE_NAME",
    ];
    for var in env_vars {
        println!("cargo:rerun-if-env-changed={}", var);
    }
}
