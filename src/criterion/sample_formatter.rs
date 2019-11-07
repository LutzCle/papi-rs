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

impl ValueFormatter for SampleFormatter {
    fn scale_values(&self, _typical_value: f64, _values: &mut [f64]) -> &'static str {
        self.event_name
    }

    fn scale_throughputs(
        &self,
        _typical_value: f64,
        _throughput: &Throughput,
        _values: &mut [f64],
    ) -> &'static str {
        self.event_name
    }

    fn scale_for_machines(&self, _values: &mut [f64]) -> &'static str {
        self.event_name
    }
}
