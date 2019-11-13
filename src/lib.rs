/*
 * Copyright 2018-2019 German Research Center for Artificial Intelligence (DFKI)
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

pub mod error;
pub mod event_set;
pub mod sampler;

#[cfg(feature = "criterion")]
pub mod criterion;

use crate::error::Result;

use papi_sys as ffi;

use error_chain::bail;
use serde_derive::Deserialize;
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
        if unsafe { ffi::PAPI_is_initialized() } != ffi::PAPI_LOW_LEVEL_INITED as i32 {
            if unsafe { ffi::PAPI_library_init(ffi::PAPI_VER_CURRENT) != ffi::PAPI_VER_CURRENT } {
                bail!("PAPI library version mismatch!");
            }
        }

        if unsafe { ffi::PAPI_thread_init(Some(libc::pthread_self)) } != ffi::PAPI_OK as i32 {
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
    ///     # use std::error::Error;
    ///     # use std::result::Result;
    ///     # use std::path::Path;
    ///     use papi::{Config, Papi};
    ///
    ///     # fn main() -> Result<(), Box<dyn Error>> {
    ///     let path = Path::new("resources/configuration.toml");
    ///     let config = Config::parse_file(path)?;
    ///     let papi = Papi::init_with_config(config)?;
    ///     #
    ///     # Ok(())
    ///     # }
    ///
    pub fn parse_file(config: &path::Path) -> Result<Self> {
        let mut input = String::new();

        fs::File::open(config).and_then(|mut f| f.read_to_string(&mut input))?;

        Self::parse_str(&input)
    }

    /// Load configuration from a string in TOML format
    ///
    ///     # use std::error::Error;
    ///     # use std::result::Result;
    ///     use papi::Config;
    ///
    ///     # fn main() -> Result<(), Box<dyn Error>> {
    ///     let config_str = r#"
    ///     [presets]
    ///     Test1 = ["UOPS_RETIRED:ALL", "UOPS_RETIRED:STALL_CYCLES"]
    ///     Test2 = ["UOPS_EXECUTED:CORE", "UOPS_EXECUTED:STALL_CYCLES"]
    ///     Test3 = ["UOPS_EXECUTED:THREAD"]
    ///     "#;
    ///
    ///     let config = Config::parse_str(&config_str)?;
    ///     #
    ///     # Ok(())
    ///     # }
    ///
    pub fn parse_str(config: &str) -> Result<Self> {
        let deserialized: Self = toml::from_str(&config)?;

        Ok(deserialized)
    }
}
