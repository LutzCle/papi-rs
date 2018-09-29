/*
 * Copyright 2018 German Research Center for Artificial Intelligence (DFKI)
 * Author: Clemens Lutz <clemens.lutz@dfki.de>
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std;
use std::fmt;
use std::result;
use std::os::raw;
use std::ffi::CStr;

use super::ffi;

pub type Result<T> = result::Result<T, Error>;

pub fn check(code: raw::c_int) -> Result<()> {

    match code as u32 {
        ffi::PAPI_OK => Ok(()),
        _ => Err(Error{ kind: ErrorKind::PapiError(code) }),
    }

}

#[derive(Debug, Clone)]
pub enum ErrorKind {
    PapiError(raw::c_int),
    InitError(&'static str),
    InvalidEvent(&'static str),
}

#[derive(Debug, Clone)]
pub struct Error {
    pub kind: ErrorKind,
}

impl ErrorKind {

    fn error_str(&self) -> &str {
        match self {
            ErrorKind::PapiError(e) => {
                let estr = unsafe {
                    let str_ptr = ffi::PAPI_strerror(*e);
                    CStr::from_ptr(str_ptr)
                        .to_str()
                        .expect("Couldn't convert error message into UTF8 string")
                };
                estr
            },
            ErrorKind::InitError(m) => m,
            ErrorKind::InvalidEvent(m) => m,
        }
    }
}

impl Error {

    pub fn new(kind: ErrorKind) -> Self {

        Self { kind }

    }

    pub fn init_error(msg: &'static str) -> Self {

        Self { kind: ErrorKind::InitError(msg) }

    }

    pub fn invalid_event(msg: &'static str) -> Self {

        Self { kind: ErrorKind::InvalidEvent(msg) }

    }
}

impl fmt::Display for Error {

    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.kind.error_str())
    }

}

impl std::error::Error for Error {

    fn description(&self) -> &str {
        self.kind.error_str()
    }

}
