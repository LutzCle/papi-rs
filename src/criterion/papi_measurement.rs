/*
 * Copyright 2019 German Research Center for Artificial Intelligence (DFKI)
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

use super::sample_formatter::SampleFormatter;
use crate::error::Result;
use crate::sampler::{ReadySampler, RunningSampler, SamplerBuilder};
use crate::Papi;
use criterion::measurement::{Measurement, ValueFormatter};

/// An adapter for Criterion that measures hardware counters
#[derive(Clone, Debug)]
pub struct PapiMeasurement {
    ready_sampler: ReadySampler,
    sample_formatter: SampleFormatter,
}

impl PapiMeasurement {
    pub fn new(papi: &Papi, event_name: &str) -> Result<Self> {
        let ready_sampler = SamplerBuilder::new(papi).add_event(event_name)?.build();
        let sample_formatter = SampleFormatter::new(event_name);

        Ok(Self {
            ready_sampler,
            sample_formatter,
        })
    }
}

impl Measurement for PapiMeasurement {
    type Intermediate = RunningSampler;
    type Value = i64;

    fn start(&self) -> Self::Intermediate {
        let ready_sampler = self.ready_sampler.clone();
        ready_sampler.start().expect("Failed to start PAPI sampler")
    }

    fn end(&self, running_sampler: Self::Intermediate) -> Self::Value {
        let sample = running_sampler.stop().expect("Failed to stop PAPI sampler");
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
