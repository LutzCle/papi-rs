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

use super::error::{check, ErrorKind, Result};
use super::ffi;
use super::Papi;

use std;
use std::fmt;
use std::mem;
use std::os::raw::{c_int, c_longlong};

/// Sampler object to sample hardware events
///
#[derive(Debug)]
pub struct ReadySampler {
    event_codes: Vec<c_int>,
}

pub struct RunningSampler {
    event_codes: Vec<c_int>,
}

/// SamplerBuilder to build a Sampler with a list of events to monitor
///
#[derive(Debug)]
pub struct SamplerBuilder<'p> {
    papi: &'p Papi,
    sampler: ReadySampler,
}

/// A Sample object contains the values collected by a Sampler
///
#[derive(Debug)]
pub struct Sample {
    event_codes: Vec<c_int>,
    values: Vec<c_longlong>,
}

impl ReadySampler {
    /// Start sampling hardware events
    ///
    pub fn start(mut self) -> Result<RunningSampler> {
        let len = self.event_codes.len() as c_int;
        check(unsafe {
            ffi::PAPI_start_counters(self.event_codes.as_mut_slice().as_mut_ptr(), len)
        })?;

        Ok(RunningSampler {
            event_codes: self.event_codes,
        })
    }
}

impl RunningSampler {
    /// Stop sampling hardware events
    ///
    /// This method destroys the Sampler object
    ///
    pub fn stop(self) -> Result<Sample> {
        let mut values = vec![0; self.event_codes.len()];

        check(unsafe {
            ffi::PAPI_stop_counters(
                values.as_mut_slice().as_mut_ptr(),
                self.event_codes.len() as c_int,
            )
        })?;

        Ok(Sample {
            event_codes: self.event_codes,
            values,
        })
    }
}

impl<'p> SamplerBuilder<'p> {
    pub fn new(papi: &'p Papi) -> Self {
        Self {
            papi,
            sampler: ReadySampler {
                event_codes: Vec::new(),
            },
        }
    }

    /// Finalize the building of a new Sampler
    ///
    pub fn build(self) -> ReadySampler {
        self.sampler
    }

    /// Add a hardware event to monitor
    ///
    ///     # extern crate papi;
    ///     # use papi::Papi;
    ///     # use papi::sampler::SamplerBuilder;
    ///     let papi = Papi::init().unwrap();
    ///     let builder = SamplerBuilder::new(&papi);
    ///     assert!(builder.add_event("CPU_CLK_UNHALTED").is_ok());
    ///
    pub fn add_event(mut self, name: &str) -> Result<Self> {
        let c_name = std::ffi::CString::new(name)
            // .or_else(|_| Err(Error::invalid_event("Invalid event name")))?;
            .or_else(|_| Err(ErrorKind::InvalidEvent("Invalid event name")))?;

        // Get event code
        let mut code: c_int = 0;
        check(unsafe { ffi::PAPI_event_name_to_code(c_name.as_ptr(), &mut code) })?;

        // Check if event is available
        check(unsafe { ffi::PAPI_query_event(code) })?;

        self.sampler.event_codes.push(code);

        Ok(self)
    }

    /// Use a preset specified in a configuration file
    ///
    ///     # extern crate papi;
    ///     # use papi::Papi;
    ///     # use papi::Config;
    ///     # use papi::sampler::SamplerBuilder;
    ///     let config_str = r#"
    ///     [presets]
    ///     Test1 = ["UOPS_RETIRED:ALL", "UOPS_RETIRED:STALL_CYCLES"]
    ///     Test2 = ["UOPS_EXECUTED:CORE", "UOPS_EXECUTED:STALL_CYCLES"]
    ///     Test3 = ["UOPS_EXECUTED:THREAD"]
    ///     "#;
    ///
    ///     let config = Config::from_str(&config_str).unwrap();
    ///     let papi = Papi::init_with_config(config).unwrap();
    ///     let builder = SamplerBuilder::new(&papi);
    ///     assert!(builder.use_preset("Test1").is_ok());
    ///
    pub fn use_preset(mut self, name: &str) -> Result<Self> {
        let maybe_config = match &self.papi.config {
            Some(o) => &o.presets,
            None => bail!("No configuration set"),
        };

        let maybe_val = match maybe_config {
            Some(c) => c.get(name),
            None => bail!("No presets configured"),
        };

        let preset = match maybe_val {
            Some(v) => v,
            None => bail!("Preset {} doesn't exist", name),
        };

        for p in preset {
            self = self.add_event(&p)?;
        }

        Ok(self)
    }
}

impl fmt::Display for Sample {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Get event_info_t and convert i8 array into UTF8 String
        let event_symbols: Vec<_> = self
            .event_codes
            .iter()
            .map(|code| {
                let mut info: ffi::PAPI_event_info_t;
                let symbol: &[u8] = unsafe {
                    info = mem::zeroed();
                    check(ffi::PAPI_get_event_info(*code, &mut info)).unwrap_or_else(|e| {
                        eprintln!("Unable to get PAPI event info, failed with {:?}", e);
                    });
                    mem::transmute(&info.symbol[..])
                };

                String::from_utf8_lossy(symbol)
            })
            .collect();

        // Print the event symbols
        event_symbols
            .iter()
            .zip(self.values.iter())
            .try_for_each(|(symbol, sample)| write!(f, "{}: {} ", symbol, sample))
    }
}

impl IntoIterator for Sample {
    type Item = (i32, i64);
    type IntoIter = ::std::iter::Zip<::std::vec::IntoIter<i32>, ::std::vec::IntoIter<i64>>;

    fn into_iter(self) -> Self::IntoIter {
        self.event_codes.into_iter().zip(self.values.into_iter())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::fmt::Write;

    #[test]
    fn complete_pipeline() {
        let papi = Papi::init().unwrap();
        let event_added = SamplerBuilder::new(&papi).add_event("CPU_CLK_UNHALTED");
        assert!(event_added.is_ok());

        let builder = event_added.unwrap();
        let ready_sampler = builder.build();
        let maybe_running = ready_sampler.start();
        assert!(maybe_running.is_ok());
        let maybe_sample = maybe_running.unwrap().stop();
        assert!(maybe_sample.is_ok());

        let sample = maybe_sample.unwrap();
        let mut buffer = String::new();
        write!(&mut buffer, "{}", &sample);

        let _all: Vec<_> = sample.into_iter().collect();
    }
}
