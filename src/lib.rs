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

extern crate libc;
extern crate papi_sys;

mod config;
pub mod error;
pub mod sampler;

use error::{Error, Result};

use papi_sys as ffi;

#[derive(Debug)]
pub struct Papi;

/// PAPI library wrapper
impl Papi {

    /// Initialize PAPI library with parallelism support
    ///
    ///     # extern crate papi;
    ///     # use papi::Papi;
    ///     assert!(Papi::init().is_ok());
    ///
    pub fn init() -> Result<Self> {

        if unsafe {
            ffi::PAPI_library_init(ffi::_papi_ver_current)
        } != ffi::_papi_ver_current
        {
            return Err(Error::init_error("PAPI library version mismatch!"))
        }

        if unsafe {
            ffi::PAPI_thread_init(Some(libc::pthread_self))
        } != ffi::PAPI_OK as i32 {
            return Err(Error::init_error("Unable to initialize PAPI threads"))
        }

        Ok(Papi)
    }
}
