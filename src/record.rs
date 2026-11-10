//! Recording chosen points over time (concept section 4): what is recorded, how often, the
//! samples. Kept in memory and backed up to the data folder; the sampling runs in the server.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

/// Points one recording holds at most.
pub const MOST_POINTS: usize = 50;
/// Samples kept per point; the oldest go first (a day at 5 s).
pub const MOST_SAMPLES: usize = 17_280;
/// Allowed sampling intervals in seconds.
pub const INTERVALS: [u64; 5] = [5, 10, 15, 30, 60];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Series {
    /// The device's key.
    pub device: String,
    /// The point's name in the device's profile.
    pub point: String,
    /// For the chart: the device's and point's names.
    pub label: String,
    pub unit: String,
    /// Milliseconds since 1970 and the value, `None` where reading failed.
    pub samples: VecDeque<(u64, Option<f64>)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Recording {
    pub series: Vec<Series>,
    pub interval: u64,
    pub running: bool,
    /// Counts the starts: the sampling of an earlier start ends itself when it wakes.
    #[serde(skip)]
    run: u64,
}

impl Default for Recording {
    fn default() -> Self {
        Recording {
            series: Vec::new(),
            interval: 10,
            running: false,
            run: 0,
        }
    }
}

impl Recording {
    /// As loaded after a restart: the samples stay, the sampling does not run on by itself.
    pub fn stopped(mut self) -> Recording {
        self.running = false;
        self
    }

    /// Starts the sampling unless it runs. Returns the number of the run to hand to the sampling.
    pub fn start(&mut self) -> Option<u64> {
        if self.running {
            return None;
        }
        self.running = true;
        self.run += 1;
        Some(self.run)
    }

    /// The sampling of this run goes on: not stopped, and not started anew since.
    pub fn runs(&self, run: u64) -> bool {
        self.running && self.run == run
    }

    pub fn add(
        &mut self,
        device: &str,
        point: &str,
        label: String,
        unit: String,
    ) -> Result<(), String> {
        if self.find(device, point).is_some() {
            return Ok(());
        }
        if self.series.len() >= MOST_POINTS {
            return Err(format!("a recording holds at most {MOST_POINTS} points"));
        }
        self.series.push(Series {
            device: device.into(),
            point: point.into(),
            label,
            unit,
            samples: VecDeque::new(),
        });
        Ok(())
    }

    pub fn remove(&mut self, device: &str, point: &str) -> bool {
        let before = self.series.len();
        self.series
            .retain(|series| !(series.device == device && series.point == point));
        self.series.len() != before
    }

    pub fn find(&self, device: &str, point: &str) -> Option<usize> {
        self.series
            .iter()
            .position(|series| series.device == device && series.point == point)
    }

    pub fn set_interval(&mut self, seconds: u64) -> Result<(), String> {
        if !INTERVALS.contains(&seconds) {
            return Err(format!("the interval must be one of {INTERVALS:?} seconds"));
        }
        self.interval = seconds;
        Ok(())
    }

    pub fn push(&mut self, index: usize, time: u64, value: Option<f64>) {
        if let Some(series) = self.series.get_mut(index) {
            if series.samples.len() >= MOST_SAMPLES {
                series.samples.pop_front();
            }
            series.samples.push_back((time, value));
        }
    }

    pub fn clear(&mut self) {
        for series in &mut self.series {
            series.samples.clear();
        }
    }

    /// Every sampling time of any series, in order, for one row each in a table.
    pub fn times(&self) -> Vec<u64> {
        let mut times: Vec<u64> = self
            .series
            .iter()
            .flat_map(|series| series.samples.iter().map(|(time, _)| *time))
            .collect();
        times.sort_unstable();
        times.dedup();
        times
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_are_recorded_once_and_samples_are_capped() {
        let mut recording = Recording::default();
        recording
            .add("tcp:a/1", "Flow", "A · Flow".into(), "°C".into())
            .expect("added");
        recording
            .add("tcp:a/1", "Flow", "A · Flow".into(), "°C".into())
            .expect("added");
        assert_eq!(recording.series.len(), 1);
        for time in 0..(MOST_SAMPLES as u64 + 5) {
            recording.push(0, time, Some(1.0));
        }
        assert_eq!(recording.series[0].samples.len(), MOST_SAMPLES);
        assert_eq!(
            recording.series[0].samples.front().map(|sample| sample.0),
            Some(5)
        );
        assert!(recording.set_interval(7).is_err());
        assert!(recording.remove("tcp:a/1", "Flow"));
    }

    #[test]
    fn a_new_start_ends_the_sampling_of_the_one_before() {
        let mut recording = Recording::default();
        let first = recording.start().expect("started");
        assert_eq!(recording.start(), None, "it runs already");
        recording.running = false;
        assert!(!recording.runs(first));
        let second = recording.start().expect("started again");
        assert!(recording.runs(second));
        assert!(!recording.runs(first), "the first sampling still sleeps");
    }
}
