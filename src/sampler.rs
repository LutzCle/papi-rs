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

use super::error::{check, ErrorKind, Result};
use super::ffi;
use super::Papi;

use error_chain::bail;
use std;
use std::fmt;
use std::marker::PhantomData;
use std::os::raw::{c_char, c_int, c_longlong};

/// A sampler that is ready to sample hardware events
///
#[derive(Clone, Debug)]
pub struct ReadySampler {
    event_codes: Vec<c_int>,
}

/// An already running sampler
///
#[derive(Debug)]
pub struct RunningSampler {
    event_codes: Vec<c_int>,
    phantom: PhantomData<*mut u8>, // unimplement Send and Sync
}

/// SamplerBuilder to build a `ReadySampler` with a list of events to monitor
///
#[derive(Clone, Debug)]
pub struct SamplerBuilder<'p> {
    papi: &'p Papi,
    event_codes: Vec<c_int>,
}

/// A Sample object contains the values collected by a sampler
///
#[derive(Clone, Debug)]
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
            phantom: PhantomData,
        })
    }
}

impl RunningSampler {
    /// Stop sampling hardware events
    ///
    /// This method destroys the sampler object
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
    /// Creates a new SampleBuilder with a PAPI instance
    ///
    pub fn new(papi: &'p Papi) -> Self {
        Self {
            papi,
            event_codes: Vec::new(),
        }
    }

    /// Finalize the building of a new sampler
    ///
    pub fn build(self) -> ReadySampler {
        ReadySampler {
            event_codes: self.event_codes,
        }
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
        // Check if there are enough hardware counters available before adding
        // another event counter
        let num_counters = unsafe { ffi::PAPI_num_counters() };
        if num_counters < 0 {
            check(num_counters)?;
        } else if self.event_codes.len() == num_counters as usize {
            Err(ErrorKind::OutOfHardwareCounters(""))?;
        }

        let c_name = std::ffi::CString::new(name)
            .or_else(|_| Err(ErrorKind::InvalidEvent("Invalid event name")))?;

        // Get event code
        let mut code: c_int = 0;
        check(unsafe { ffi::PAPI_event_name_to_code(c_name.as_ptr(), &mut code) })?;

        // Check if event is available
        check(unsafe { ffi::PAPI_query_event(code) })?;

        self.event_codes.push(code);

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
    ///     let config = Config::parse_str(&config_str).unwrap();
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

impl Sample {
    #[allow(dead_code)]
    pub(crate) fn event_code_to_name(event_code: c_int) -> Result<String> {
        let mut c_event_name = [0_u8; ffi::PAPI_MAX_STR_LEN as usize];
        check(unsafe {
            ffi::PAPI_event_code_to_name(
                event_code,
                c_event_name.as_mut_ptr() as *mut u8 as *mut c_char,
            )
        })?;

        unsafe { Ok(String::from_utf8_unchecked(c_event_name.to_vec())) }
    }
}

impl fmt::Display for Sample {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Get event_info_t and convert i8 array into UTF8 String
        let event_symbols = self
            .event_codes
            .iter()
            .map(|&code| Self::event_code_to_name(code).map_err(|_| fmt::Error::default()))
            .collect::<std::result::Result<Vec<String>, fmt::Error>>()?;

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
        write!(&mut buffer, "{}", &sample).unwrap();

        let _all: Vec<_> = sample.into_iter().collect();
    }
}
