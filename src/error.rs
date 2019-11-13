// Copyright 2018-2019 German Research Center for Artificial Intelligence (DFKI)
// Author: Clemens Lutz <clemens.lutz@dfki.de>
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use error_chain::error_chain;
use std::ffi::CStr;
use std::os::raw::c_int;

use super::ffi;

// pub type Result<T> = result::Result<T, Error>;
//
pub fn check(code: c_int) -> Result<()> {
    match code as u32 {
        ffi::PAPI_OK => Ok(()),
        _ => Err(ErrorKind::PapiError(code).into()),
    }
}

error_chain! {
    errors {
        PapiError(e: c_int) {
            description("PAPI command failed")
            display("PAPI command returned with: '{}'",
                        unsafe {
                            let str_ptr = ffi::PAPI_strerror(*e);
                            CStr::from_ptr(str_ptr)
                                .to_str()
                                .expect("Couldn't convert error message into UTF8 string")
                        }
                    )
        }
        InvalidEvent(e: &'static str) {
            description("invalid event name")
            display("invalid event name: '{}'", e)
        }
        InvalidArgument(e: String) {
            description("invalid argument")
            display("invalid argument: '{}'", e)
        }
        OutOfHardwareCounters(e: &'static str) {
            description("out of hardware counters")
            display("out of hardware counters")
        }
    }

    foreign_links {
        Io(::std::io::Error);
        TomlDe(toml::de::Error);
    }
}
