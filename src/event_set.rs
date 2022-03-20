// Copyright 2019 German Research Center for Artificial Intelligence (DFKI)
// Author: Clemens Lutz <clemens.lutz@dfki.de>
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

//! A Rust-ified wrapper around the PAPI low-level API.
//!
//! Provides a low-overhead Rust API for PAPI event sets. Event sets are
//! user-defined groups of hardware events, that are used to sample hardware
//! event counters.
//!
//! # Cloning
//!
//! The `EventSetBuilder` and `ReadyEventSet` can cloned with `try_clone`.
//! However, the user must ensure that the hardware has sufficient counter
//! resources to handle multiple event set instances (i.e., `try_clone` or
//! `start` can return an error).
//!
//! The use-case is typically that multiple threads running on different CPUs
//! can run identical event sets.
//!
//! In contrast, a `RunningEventSet` cannot be cloned, because the semantics of
//! cloning PAPI-internal counters is unclear.
//!
//! # Thread Safety
//!
//! Event sets and the builder are bound to a thread. The reason is that PAPI
//! holds internal state for each event set, and the PAPI documentation is
//! unclear whether this state can be transferred between threads.
//!
//! # Examples
//!
//!      # use std::error::Error;
//!      # use std::result::Result;
//!      use papi::event_set::{EventSetBuilder, Sample};
//!      #
//!      # fn main() -> Result<(), Box<dyn Error>> {
//!
//!      // Initialize the PAPI library
//!      let papi = papi::Papi::init()?;
//!
//!      // Create an event set
//!      let ready_event_set = papi::event_set::EventSetBuilder::new(&papi)?
//!          .add_event_by_name("CPU_CLK_UNHALTED")?
//!          .build()?;
//!
//!      // Create and initialize a sample
//!      let mut sample = Sample::default();
//!      ready_event_set.init_sample(&mut sample)?;
//!
//!      // Run the event set
//!      let running_event_set = ready_event_set.start()?;
//!
//!      // Do some work
//!      work();
//!
//!      // Stop the event set and collect the result values
//!      running_event_set.stop(&mut sample)?;
//!      println!("{}", sample);
//!      # Ok(())
//!      # }
//!      #
//!      # fn work() {
//!      #     let collected: u32 = (0..100).map(|x| x * 2).filter(|x| x % 3 == 0).sum();
//!      #     println!("Summed up {}", collected);
//!      # }

use super::error::{check, ErrorKind, Result};
use super::ffi;
use super::Papi;
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::num::NonZeroU16;
use std::os::raw::c_char;
use std::ptr;

/// An event set that is ready to sample hardware events.
#[derive(Debug)]
pub struct ReadyEventSet {
    event_set: Option<i32>,
    event_set_hash: u64,
    num_events: NonZeroU16,
    phantom: PhantomData<*mut u8>, // unimplement Send and Sync
}

/// An already running event set.
#[derive(Debug)]
pub struct RunningEventSet {
    event_set: Option<i32>,
    event_set_hash: u64,
    num_events: NonZeroU16,
    phantom: PhantomData<*mut u8>, // unimplement Send and Sync
}

/// A builder that builds a `ReadyEventSet` with a list of hardware events to
/// monitor.
#[derive(Debug)]
pub struct EventSetBuilder<'p> {
    papi: &'p Papi,
    event_set: Option<i32>,
    num_events: u16,
    phantom: PhantomData<*mut u8>, // unimplement Send and Sync
}

/// A collection of sampled hardware event values.
///
///     # use std::error::Error;
///     # use std::result::Result;
///     # use papi::Papi;
///     # use papi::event_set::{EventSetBuilder, Sample};
///     #
///     # fn main() -> Result<(), Box<dyn Error>> {
///     # let papi = Papi::init()?;
///     # let ready_event_set = EventSetBuilder::new(&papi)?
///     #     .add_event_by_name("CPU_CLK_UNHALTED")?
///     #     .build()?;
///     #
///     let mut sample = Sample::default();
///
///     ready_event_set.init_sample(&mut sample)?;
///     let running_event_set = ready_event_set.start()?;
///     running_event_set.stop(&mut sample)?;
///
///     println!("Sample: {}", sample);
///     let values: Vec<(String, i64)> = sample.into_iter().collect();
///     #
///     # Ok(())
///     # }
///
#[derive(Clone, Debug)]
pub struct Sample {
    event_set_hash: u64,
    event_codes: Vec<i32>,
    values: Vec<i64>,
}

impl ReadyEventSet {
    /// Starts sampling the hardware events specified by the event set.
    ///
    ///     # use std::error::Error;
    ///     # use std::result::Result;
    ///     # use papi::Papi;
    ///     # use papi::event_set::{EventSetBuilder, Sample};
    ///     #
    ///     # fn main() -> Result<(), Box<dyn Error>> {
    ///     # let papi = Papi::init()?;
    ///     # let ready_event_set = EventSetBuilder::new(&papi)?
    ///     #     .add_event_by_name("CPU_CLK_UNHALTED")?
    ///     #     .build()?;
    ///     #
    ///     # let mut sample = Sample::default();
    ///     # ready_event_set.init_sample(&mut sample)?;
    ///     let running_event_set = ready_event_set.start()?;
    ///     # running_event_set.stop(&mut sample)?;
    ///     # Ok(())
    ///     # }
    ///
    pub fn start(mut self) -> Result<RunningEventSet> {
        unsafe {
            check(ffi::PAPI_start(self.event_set.unwrap()))?;
        }

        Ok(RunningEventSet {
            event_set: self.event_set.take(),
            event_set_hash: self.event_set_hash,
            num_events: self.num_events,
            phantom: PhantomData,
        })
    }

    /// Initializes a `Sample` for use with the current event set.
    ///
    /// This is required before passing a `Sample` to any other event set
    /// funtions.
    ///
    /// Mandatory initialization of the `Sample` moves all allocations and other
    /// setup tasks out of the hot path, leading to less overhead during
    /// measurements.
    ///
    ///     # use std::error::Error;
    ///     # use std::result::Result;
    ///     # use papi::Papi;
    ///     # use papi::event_set::{EventSetBuilder, Sample};
    ///     #
    ///     # fn main() -> Result<(), Box<dyn Error>> {
    ///     # let papi = Papi::init()?;
    ///     # let ready_event_set = EventSetBuilder::new(&papi)?
    ///     #     .add_event_by_name("CPU_CLK_UNHALTED")?
    ///     #     .build()?;
    ///     #
    ///     let mut sample = Sample::default();
    ///     ready_event_set.init_sample(&mut sample)?;
    ///     #
    ///     # Ok(())
    ///     # }
    ///
    pub fn init_sample(&self, sample: &mut Sample) -> Result<()> {
        let num_events = self.num_events.get().into();
        let mut num_events_ffi = self.num_events.get().into();
        let event_set = self
            .event_set
            .expect("EventSet uninitialized; looks like a bug");

        sample.event_set_hash = self.event_set_hash;

        sample.event_codes.clear();
        sample.event_codes.resize(num_events, 0);

        sample.values.clear();
        sample.values.resize(num_events, 0);

        unsafe {
            check(ffi::PAPI_list_events(
                event_set,
                sample.event_codes.as_mut_ptr(),
                &mut num_events_ffi,
            ))?;
        }

        Ok(())
    }

    /// Creates a new, distinct `ReadyEventSet` instance containing the same
    /// events as the given `ReadyEventSet` instance.
    ///
    ///     # use std::error::Error;
    ///     # use std::result::Result;
    ///     # use papi::Papi;
    ///     # use papi::event_set::{EventSetBuilder, Sample};
    ///     #
    ///     # fn main() -> Result<(), Box<dyn Error>> {
    ///     # let papi = Papi::init()?;
    ///     # let ready_event_set = EventSetBuilder::new(&papi)?
    ///     #     .add_event_by_name("CPU_CLK_UNHALTED")?
    ///     #     .build()?;
    ///     #
    ///     let cloned_event_set = ready_event_set.try_clone()?;
    ///     # Ok(())
    ///     # }
    ///
    pub fn try_clone(&self) -> Result<Self> {
        let num_events = self.num_events.get().into();
        let mut num_events_ffi = self.num_events.get().into();
        let event_set = self
            .event_set
            .expect("EventSet uninitialized; looks like a bug");
        let mut new_event_set = ffi::PAPI_NULL;
        let mut event_codes = vec![0; num_events];

        unsafe {
            check(ffi::PAPI_list_events(
                event_set,
                event_codes.as_mut_ptr(),
                &mut num_events_ffi,
            ))?;
            check(ffi::PAPI_create_eventset(&mut new_event_set))?;
            check(ffi::PAPI_add_events(
                new_event_set,
                event_codes.as_mut_ptr(),
                num_events_ffi,
            ))?;
        }

        Ok(ReadyEventSet {
            event_set: Some(new_event_set),
            event_set_hash: self.event_set_hash,
            num_events: self.num_events,
            phantom: PhantomData,
        })
    }
}

impl Drop for ReadyEventSet {
    fn drop(&mut self) {
        if let Some(ref mut es) = self.event_set.take() {
            unsafe {
                check(ffi::PAPI_cleanup_eventset(*es)).expect("Failed to cleanup PAPI event set");
                check(ffi::PAPI_destroy_eventset(es)).expect("Failed to destroy PAPI event set");
            }
        }
    }
}

impl RunningEventSet {
    /// Accumulates the hardware events sepecified by the event set onto the
    /// given `Sample`.
    ///
    /// The hardware counters are reset and continue running after the
    /// accumulation.
    ///
    ///     # use std::error::Error;
    ///     # use std::result::Result;
    ///     # use papi::Papi;
    ///     # use papi::event_set::{EventSetBuilder, Sample};
    ///     #
    ///     # fn main() -> Result<(), Box<dyn Error>> {
    ///     # let papi = Papi::init()?;
    ///     # let ready_event_set = EventSetBuilder::new(&papi)?
    ///     #     .add_event_by_name("CPU_CLK_UNHALTED")?
    ///     #     .build()?;
    ///     #
    ///     let mut sample = Sample::default();
    ///     ready_event_set.init_sample(&mut sample)?;
    ///     let running_event_set = ready_event_set.start()?;
    ///
    ///     running_event_set.accum(&mut sample)?;
    ///     #
    ///     # running_event_set.stop(&mut sample)?;
    ///     # Ok(())
    ///     # }
    ///
    pub fn accum(&self, sample: &mut Sample) -> Result<()> {
        let event_set = self
            .event_set
            .expect("EventSet uninitialized; looks like a bug");

        if sample.event_set_hash != self.event_set_hash {
            Err(ErrorKind::InvalidArgument(
                "Sample is not initialized".into(),
            ))?;
        }

        unsafe {
            check(ffi::PAPI_accum(event_set, sample.values.as_mut_ptr()))?;
        }

        Ok(())
    }

    /// Reads the hardware events sepecified by the event set.
    ///
    /// The hardware counters continue running after the read.
    ///
    ///     # use std::error::Error;
    ///     # use std::result::Result;
    ///     # use papi::Papi;
    ///     # use papi::event_set::{EventSetBuilder, Sample};
    ///     #
    ///     # fn main() -> Result<(), Box<dyn Error>> {
    ///     # let papi = Papi::init()?;
    ///     # let ready_event_set = EventSetBuilder::new(&papi)?
    ///     #     .add_event_by_name("CPU_CLK_UNHALTED")?
    ///     #     .build()?;
    ///     #
    ///     let mut sample = Sample::default();
    ///     ready_event_set.init_sample(&mut sample)?;
    ///     let running_event_set = ready_event_set.start()?;
    ///
    ///     running_event_set.read(&mut sample)?;
    ///     #
    ///     # running_event_set.stop(&mut sample)?;
    ///     # Ok(())
    ///     # }
    ///
    pub fn read(&self, sample: &mut Sample) -> Result<()> {
        let event_set = self
            .event_set
            .expect("EventSet uninitialized; looks like a bug");

        if sample.event_set_hash != self.event_set_hash {
            Err(ErrorKind::InvalidArgument(
                "Sample is not initialized".into(),
            ))?;
        }

        unsafe {
            check(ffi::PAPI_read(event_set, sample.values.as_mut_ptr()))?;
        }

        Ok(())
    }

    /// Stops sampling the hardware events specified by the event set.
    ///
    /// Note that this method destroys the event set.
    ///
    ///     # use std::error::Error;
    ///     # use std::result::Result;
    ///     # use papi::Papi;
    ///     # use papi::event_set::{EventSetBuilder, Sample};
    ///     #
    ///     # fn main() -> Result<(), Box<dyn Error>> {
    ///     # let papi = Papi::init()?;
    ///     # let ready_event_set = EventSetBuilder::new(&papi)?
    ///     #     .add_event_by_name("CPU_CLK_UNHALTED")?
    ///     #     .build()?;
    ///     #
    ///     let mut sample = Sample::default();
    ///     ready_event_set.init_sample(&mut sample)?;
    ///     let running_event_set = ready_event_set.start()?;
    ///
    ///     running_event_set.stop(&mut sample)?;
    ///     #
    ///     # Ok(())
    ///     # }
    ///
    pub fn stop(self, sample: &mut Sample) -> Result<()> {
        let event_set = self
            .event_set
            .expect("EventSet uninitialized; looks like a bug");

        if sample.event_set_hash != self.event_set_hash {
            Err(ErrorKind::InvalidArgument(
                "Sample is not initialized".into(),
            ))?;
        }

        unsafe {
            check(ffi::PAPI_stop(event_set, sample.values.as_mut_ptr()))?;
        }

        Ok(())
    }
}

impl Drop for RunningEventSet {
    fn drop(&mut self) {
        if let Some(ref mut es) = self.event_set.take() {
            unsafe {
                let mut state = 0;
                check(ffi::PAPI_state(*es, &mut state)).expect("Failed to get PAPI counter state");
                if (state as u32 & ffi::PAPI_RUNNING) != 0 {
                    check(ffi::PAPI_stop(*es, ptr::null_mut()))
                        .expect("Failed to stop PAPI counters");
                }

                check(ffi::PAPI_cleanup_eventset(*es)).expect("Failed to cleanup PAPI event set");
                check(ffi::PAPI_destroy_eventset(es)).expect("Failed to destroy PAPI event set");
            }
        }
    }
}

impl<'p> EventSetBuilder<'p> {
    /// Creates a new EventSetBuilder.
    ///
    /// This requires an initialized PAPI instance.
    ///
    ///     # use std::error::Error;
    ///     # use std::result::Result;
    ///     # use papi::Papi;
    ///     # use papi::event_set::EventSetBuilder;
    ///     #
    ///     # fn main() -> Result<(), Box<dyn Error>> {
    ///     # let papi = Papi::init()?;
    ///     let builder = EventSetBuilder::new(&papi)?;
    ///     # assert!(builder.add_event_by_name("CPU_CLK_UNHALTED").is_ok());
    ///     # Ok(())
    ///     # }
    ///
    pub fn new(papi: &'p Papi) -> Result<Self> {
        let mut event_set = ffi::PAPI_NULL;

        unsafe {
            check(ffi::PAPI_create_eventset(&mut event_set))?;
        }

        Ok(Self {
            papi,
            event_set: Some(event_set),
            num_events: 0,
            phantom: PhantomData,
        })
    }

    /// Finalizes the building of a new `ReadyEventSet`.
    ///
    ///     # use std::error::Error;
    ///     # use std::result::Result;
    ///     # use papi::Papi;
    ///     # use papi::event_set::EventSetBuilder;
    ///     #
    ///     # fn main() -> Result<(), Box<dyn Error>> {
    ///     # let papi = Papi::init()?;
    ///     # let builder = EventSetBuilder::new(&papi)?
    ///     #     .add_event_by_name("CPU_CLK_UNHALTED")?;
    ///     #
    ///     let ready_event_set = builder.build()?;
    ///     #
    ///     # Ok(())
    ///     # }
    ///
    pub fn build(mut self) -> Result<ReadyEventSet> {
        let num_events = NonZeroU16::new(self.num_events).ok_or_else(|| {
            ErrorKind::InvalidArgument("Cannot create EventSet without events!".into())
        })?;
        let mut num_events_ffi = self.num_events.into();
        let mut event_codes = vec![0; self.num_events.into()];

        let event_set = self
            .event_set
            .expect("EventSet uninitialized; looks like a bug");

        unsafe {
            check(ffi::PAPI_list_events(
                event_set,
                event_codes.as_mut_ptr(),
                &mut num_events_ffi,
            ))?;
        }

        let mut hasher = DefaultHasher::new();
        event_codes.iter().for_each(|code| code.hash(&mut hasher));
        let event_set_hash = hasher.finish();

        Ok(ReadyEventSet {
            event_set: self.event_set.take(),
            event_set_hash,
            num_events,
            phantom: PhantomData,
        })
    }

    /// Adds a hardware event specified by its name to the event set.
    ///
    ///     # use std::error::Error;
    ///     # use std::result::Result;
    ///     # use papi::Papi;
    ///     # use papi::event_set::EventSetBuilder;
    ///     #
    ///     # fn main() -> Result<(), Box<dyn Error>> {
    ///     # let papi = Papi::init()?;
    ///     # let builder = EventSetBuilder::new(&papi)?;
    ///     #
    ///     builder.add_event_by_name("CPU_CLK_UNHALTED")?;
    ///     #
    ///     # Ok(())
    ///     # }
    ///
    pub fn add_event_by_name(mut self, name: &str) -> Result<Self> {
        // Check if there are enough hardware counters available before adding
        // another event counter
        let num_events = unsafe { ffi::PAPI_num_events(self.event_set.unwrap()) };
        if num_events < 0 {
            check(num_events)?;
        }
        let num_counters = unsafe { ffi::PAPI_num_cmp_hwctrs(0) };
        if num_counters < 0 {
            check(num_counters)?;
        } else if num_events == num_counters {
            Err(ErrorKind::OutOfHardwareCounters(
                "Too many hardware events specified",
            ))?;
        }

        let c_name = std::ffi::CString::new(name)
            .or_else(|_| Err(ErrorKind::InvalidEvent("Invalid event name")))?;

        // Get event code
        let mut code: i32 = 0;
        unsafe {
            check(ffi::PAPI_event_name_to_code(c_name.as_ptr(), &mut code))?;
            check(ffi::PAPI_add_event(self.event_set.unwrap(), code))?;
        }

        self.num_events += 1;

        Ok(self)
    }

    /// Adds the events from a preset to the event set.
    ///
    /// Presets are specified by the configuration file.
    ///
    ///     # use std::error::Error;
    ///     # use std::result::Result;
    ///     # use papi::Papi;
    ///     use papi::Config;
    ///     # use papi::event_set::EventSetBuilder;
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
    ///     # let papi = Papi::init_with_config(config)?;
    ///     # let builder = EventSetBuilder::new(&papi)?;
    ///     builder.use_preset("Test1")?;
    ///     #
    ///     # Ok(())
    ///     # }
    ///
    pub fn use_preset(mut self, name: &str) -> Result<Self> {
        let maybe_config = match &self.papi.config {
            Some(o) => &o.presets,
            None => Err(ErrorKind::InvalidArgument("No configuration set".into()))?,
        };

        let maybe_val = match maybe_config {
            Some(c) => c.get(name),
            None => Err(ErrorKind::InvalidArgument("No presets configured".into()))?,
        };

        let preset = match maybe_val {
            Some(v) => v,
            None => Err(ErrorKind::InvalidArgument(format!(
                "Preset {} doesn't exist",
                name
            )))?,
        };

        for p in preset {
            self = self.add_event_by_name(&p)?;
        }

        Ok(self)
    }

    /// Creates a new, distinct `EventSetBuilder` instance containing the same
    /// events as the given `EventSetBuilder` instance.
    ///
    /// WARNING: Not yet implemented.
    pub fn try_clone(&self) -> Result<Self> {
        unimplemented!();
    }
}

impl Drop for EventSetBuilder<'_> {
    fn drop(&mut self) {
        if let Some(ref mut es) = self.event_set.take() {
            unsafe {
                check(ffi::PAPI_cleanup_eventset(*es)).expect("Failed to cleanup PAPI event set");
                check(ffi::PAPI_destroy_eventset(es)).expect("Failed to destroy PAPI event set");
            }
        }
    }
}

impl Sample {
    /// Converts a PAPI event code to a code name string.
    pub(crate) fn event_code_to_name(event_code: i32) -> Result<String> {
        let mut c_event_name = [0_u8; ffi::PAPI_MAX_STR_LEN as usize];

        unsafe {
            check(ffi::PAPI_event_code_to_name(
                event_code,
                c_event_name.as_mut_ptr() as *mut u8 as *mut c_char,
            ))?;
        }
        let nul_index = c_event_name
            .iter()
            .position(|&byte| byte == 0)
            .expect("Couldn't find '\0' byte in PAPI event codename");

        Ok(unsafe { String::from_utf8_unchecked(c_event_name[0..nul_index].to_vec()) })
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

impl Default for Sample {
    fn default() -> Self {
        Sample {
            event_set_hash: Default::default(),
            event_codes: Vec::new(),
            values: Vec::new(),
        }
    }
}

impl IntoIterator for Sample {
    type Item = (String, i64);
    type IntoIter = ::std::iter::Zip<::std::vec::IntoIter<String>, ::std::vec::IntoIter<i64>>;

    fn into_iter(self) -> Self::IntoIter {
        let event_names: Vec<_> = self
            .event_codes
            .into_iter()
            .map(|code| {
                Self::event_code_to_name(code)
                    .expect("Failed to convert event code into event name string")
            })
            .collect();
        event_names.into_iter().zip(self.values.into_iter())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::fmt::Write;

    #[test]
    fn complete_pipeline() {
        let papi = Papi::init().unwrap();
        let event_added = EventSetBuilder::new(&papi)
            .unwrap()
            .add_event_by_name("CPU_CLK_UNHALTED");
        assert!(event_added.is_ok());

        let builder = event_added.unwrap();
        let ready_event_set = builder.build().unwrap();
        let mut sample = Sample::default();
        ready_event_set.init_sample(&mut sample).unwrap();
        let running = ready_event_set.start().unwrap();
        running.stop(&mut sample).unwrap();

        let mut buffer = String::new();
        write!(&mut buffer, "{}", &sample).unwrap();

        let _all: Vec<(String, i64)> = sample.into_iter().collect();
    }

    #[test]
    #[ignore]
    fn run_two_event_set_instances() {
        let papi = Papi::init().unwrap();
        let event_added = EventSetBuilder::new(&papi)
            .unwrap()
            .add_event_by_name("CPU_CLK_UNHALTED");
        assert!(event_added.is_ok());

        let builder = event_added.unwrap();
        let ready_event_set = builder.build().unwrap();
        let cloned_event_set = ready_event_set.try_clone().unwrap();

        let mut sample_1 = Sample::default();
        let mut sample_2 = Sample::default();
        ready_event_set.init_sample(&mut sample_1).unwrap();
        cloned_event_set.init_sample(&mut sample_2).unwrap();

        let running = ready_event_set.start().unwrap();
        let running_cloned = cloned_event_set.start().unwrap();

        running.stop(&mut sample_1).unwrap();
        running_cloned.stop(&mut sample_1).unwrap();
    }

    #[test]
    fn drop_unbuilt_event_set_builder() {
        let papi = Papi::init().unwrap();
        let event_added = EventSetBuilder::new(&papi)
            .unwrap()
            .add_event_by_name("CPU_CLK_UNHALTED");
        assert!(event_added.is_ok());
    }

    #[test]
    fn drop_unrun_ready_event_set() {
        let papi = Papi::init().unwrap();
        let event_added = EventSetBuilder::new(&papi)
            .unwrap()
            .add_event_by_name("CPU_CLK_UNHALTED");
        assert!(event_added.is_ok());

        let builder = event_added.unwrap();
        assert!(builder.build().is_ok());
    }

    #[test]
    fn drop_unstopped_running_event_set() {
        let papi = Papi::init().unwrap();
        let event_added = EventSetBuilder::new(&papi)
            .unwrap()
            .add_event_by_name("CPU_CLK_UNHALTED");
        assert!(event_added.is_ok());

        let builder = event_added.unwrap();
        let ready_event_set = builder.build().unwrap();
        let mut sample = Sample::default();
        ready_event_set.init_sample(&mut sample).unwrap();
        assert!(ready_event_set.start().is_ok());
    }
}
