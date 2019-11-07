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

use criterion::measurement::ValueFormatter;
use criterion::Throughput;

/// An adapter for Criterion that formats PAPI samples
#[derive(Clone, Debug)]
pub(crate) struct SampleFormatter {
    event_name: &'static str,
}

impl SampleFormatter {
    /// Creates a new SampleFormatter containing an event name
    pub(crate) fn new(event_name: &'static str) -> Self {
        Self { event_name }
    }
}

impl SampleFormatter {
    /// Calculates throughput in bytes per event
    fn bytes_per_event(&self, bytes: f64, values: &mut [f64]) -> &'static str {
        values.iter_mut().for_each(|val| *val = bytes / *val);
        "Bytes/event"
    }

    /// Calculates throughput in elements per event
    fn elements_per_event(&self, elems: f64, values: &mut [f64]) -> &'static str {
        values.iter_mut().for_each(|val| *val = elems / *val);
        "elems/event"
    }
}

impl ValueFormatter for SampleFormatter {
    fn scale_values(&self, _typical_value: f64, _values: &mut [f64]) -> &'static str {
        self.event_name
    }

    fn scale_throughputs(
        &self,
        _typical_value: f64,
        throughput: &Throughput,
        values: &mut [f64],
    ) -> &'static str {
        match *throughput {
            Throughput::Bytes(bytes) => self.bytes_per_event(bytes as f64, values),
            Throughput::Elements(elems) => self.elements_per_event(elems as f64, values),
        }
    }

    fn scale_for_machines(&self, _values: &mut [f64]) -> &'static str {
        self.event_name
    }
}
