// Copyright 2019 German Research Center for Artificial Intelligence (DFKI)
// Author: Clemens Lutz <clemens.lutz@dfki.de>
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use super::sample_formatter::SampleFormatter;
use crate::error::Result;
use crate::event_set::{EventSetBuilder, ReadyEventSet, RunningEventSet, Sample};
use crate::Papi;
use criterion::measurement::{Measurement, ValueFormatter};

/// An adapter for Criterion that measures hardware counters
#[derive(Clone, Debug)]
pub struct PapiMeasurement {
    ready_event_set: CloneableEventSet,
    sample: Sample,
    sample_formatter: SampleFormatter,
}

impl PapiMeasurement {
    pub fn new(papi: &Papi, event_name: &'static str) -> Result<Self> {
        let ready_event_set = EventSetBuilder::new(papi)?
            .add_event_by_name(event_name)?
            .build()?;
        let mut sample = Sample::default();
        ready_event_set.init_sample(&mut sample)?;
        let sample_formatter = SampleFormatter::new(event_name);

        Ok(Self {
            ready_event_set: CloneableEventSet(ready_event_set),
            sample,
            sample_formatter,
        })
    }
}

impl Measurement for PapiMeasurement {
    type Intermediate = RunningEventSet;
    type Value = i64;

    fn start(&self) -> Self::Intermediate {
        let ready_event_set = self.ready_event_set.clone().0;
        ready_event_set
            .start()
            .expect("Failed to start PAPI event set")
    }

    fn end(&self, running_event_set: Self::Intermediate) -> Self::Value {
        let mut sample = self.sample.clone();
        running_event_set
            .stop(&mut sample)
            .expect("Failed to stop PAPI event set");
        sample
            .into_iter()
            .nth(0)
            .expect("Failed to get a value from PAPI sample; is the sample empty?")
            .1
    }

    fn add(&self, v1: &Self::Value, v2: &Self::Value) -> Self::Value {
        *v1 + *v2
    }

    fn zero(&self) -> Self::Value {
        0
    }

    fn to_f64(&self, value: &Self::Value) -> f64 {
        *value as f64
    }

    fn formatter(&self) -> &dyn ValueFormatter {
        &self.sample_formatter
    }
}

#[derive(Debug)]
struct CloneableEventSet(ReadyEventSet);

impl Clone for CloneableEventSet {
    fn clone(&self) -> Self {
        Self(self.0.try_clone().expect("Failed to clone PAPI event set"))
    }
}
