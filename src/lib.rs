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
extern crate serde;
extern crate toml;

#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate serde_derive;

pub mod error;
pub mod sampler;

use error::Result;

use papi_sys as ffi;

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path;

#[derive(Debug)]
pub struct Papi {
    config: Option<Config>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    presets: Option<BTreeMap<String, Vec<String>>>,
}

/// PAPI library wrapper
impl Papi {
    /// Initialize PAPI library with parallelism support
    ///
    ///     # extern crate papi;
    ///     # use papi::Papi;
    ///     assert!(Papi::init().is_ok());
    ///
    pub fn init() -> Result<Self> {
        if unsafe { ffi::PAPI_library_init(ffi::_papi_ver_current) } != ffi::_papi_ver_current {
            // return Err(Error::init_error("PAPI library version mismatch!"))
            bail!("PAPI library version mismatch!");
        }

        if unsafe { ffi::PAPI_thread_init(Some(libc::pthread_self)) } != ffi::PAPI_OK as i32 {
            // return Err(Error::init_error("Unable to initialize PAPI threads"))
            bail!("Unable to initialize PAPI threads");
        }

        Ok(Papi { config: None })
    }

    pub fn init_with_config(config: Config) -> Result<Self> {
        let mut papi = Self::init()?;
        papi.config = Some(config);
        Ok(papi)
    }
}

impl Config {
    /// Load configuration file in TOML format
    ///
    pub fn from_path(config: &path::Path) -> Result<Self> {
        let mut input = String::new();

        fs::File::open(config).and_then(|mut f| f.read_to_string(&mut input))?;

        Self::from_str(&input)
    }

    /// Load configuration from a string in TOML format
    ///
    ///     # extern crate papi;
    ///     # use papi::Config;
    ///     let config_str = r#"
    ///     [presets]
    ///     Test1 = ["UOPS_RETIRED:ALL", "UOPS_RETIRED:STALL_CYCLES"]
    ///     Test2 = ["UOPS_EXECUTED:CORE", "UOPS_EXECUTED:STALL_CYCLES"]
    ///     Test3 = ["UOPS_EXECUTED:THREAD"]
    ///     "#;
    ///
    ///     let config = Config::from_str(&config_str);
    ///     assert!(config.is_ok());
    ///
    pub fn from_str(config: &str) -> Result<Self> {
        let deserialized: Self = toml::from_str(&config)?;

        Ok(deserialized)
    }
}
