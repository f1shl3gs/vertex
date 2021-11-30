use std::fmt::{Display, Formatter};
use std::ops::Deref;
use std::time::Instant;


/// A WeightedSum contains an averaging mechanism that accepts a varying weight at each
/// point to be averaged, and biases the mean based on those weights
#[derive(Clone, Copy, Debug, Default)]
pub struct WeightedSum {
    total: f64,
    weights: f64,
    min: Option<f64>,
    max: Option<f64>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WeightedSumStats {
    pub min: f64,
    pub max: f64,
    pub mean: f64,
}

impl WeightedSum {
    pub fn add(&mut self, value: f64, weight: f64) {
        self.total += value * weight;
        self.weights += weight;
        self.max = Some(opt_max(self.max, value));
        self.min = Some(opt_min(self.min, value));
    }

    pub fn mean(&self) -> Option<f64> {
        if self.weights == 0.0 {
            None
        } else {
            Some(self.total / self.weights)
        }
    }

    pub fn stats(&self) -> Option<WeightedSumStats> {
        self.mean()
            .map(|mean| WeightedSumStats {
                mean,
                min: self.min.unwrap(),
                max: self.max.unwrap(),
            })
    }
}

impl Display for WeightedSum {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::fmt::Result {
        match self.stats() {
            None => write!(fmt, "[No stats]"),
            Some(stats) => write!(
                fmt,
                "[min={}, max={}, mean={}]",
                stats.min, stats.max, stats.mean
            ),
        }
    }
}

fn opt_max(opt: Option<f64>, value: f64) -> f64 {
    match opt {
        None => value,
        Some(v) if v > value => v,
        _ => value
    }
}

fn opt_min(opt: Option<f64>, value: f64) -> f64 {
    match opt {
        None => value,
        Some(v) if v < value => v,
        _ => value
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct HistogramStats {
    // The first bucket with a value
    pub min: usize,
    // The last bucket with a value
    pub max: usize,
    // The bucket with the highest value
    pub mode: usize,
    // The total over all the weights
    pub total: f64,
    // The mean of all indices weighted by their value
    pub mean: f64,
}

#[derive(Clone, Debug, Default)]
pub struct Histogram {
    totals: Vec<f64>,
}

impl Histogram {
    pub fn add(&mut self, index: usize, amount: f64) {
        if self.totals.len() <= index {
            self.totals
                .extend((self.totals.len()..index + 1).map(|_| 0.0));
        }

        self.totals[index] += amount;
    }

    pub fn stats(&self) -> Option<HistogramStats> {
        let (min, max, mode, sum) = self.totals
            .iter()
            .enumerate()
            .fold(
                (None, None, None, WeightedSum::default()),
                |(mut min, mut max, mut mode, mut sum), (i, &total)| {
                    if total > 0.0 {
                        min = min.or(Some(i));
                        max = Some(i);
                        mode = Some(match mode {
                            None => (i, total),
                            Some((index, value)) => {
                                if value > total {
                                    (index, value)
                                } else {
                                    (i, total)
                                }
                            }
                        });
                    }

                    sum.add(i as f64, total);
                    (min, max, mode, sum)
                },
            );

        min.map(|_| HistogramStats {
            min: min.unwrap(),
            max: max.unwrap(),
            mode: mode.unwrap().0,
            mean: sum.mean().unwrap(),
            total: sum.weights,
        })
    }
}

impl Display for Histogram {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::fmt::Result {
        match self.stats() {
            None => write!(fmt, "[No stats]"),
            Some(stats) => write!(
                fmt,
                "[min={}, max={}, mode={}, mean={}, total={}]",
                stats.min, stats.max, stats.mode, stats.mean, stats.total
            )
        }
    }
}

/// A TimeHistogram is a Histogram where the weights are equal to the length of time
/// since the last item was added. Time between the start of the program and the first
/// `add` is ignored.
#[derive(Clone, Debug, Default)]
pub struct TimeHistogram {
    histogram: Histogram,
    last_time: Option<Instant>,
}

impl TimeHistogram {
    pub fn add(&mut self, index: usize, instant: Instant) {
        if let Some(last) = self.last_time {
            let duration = instant.saturating_duration_since(last).as_secs_f64();
            self.histogram.add(index, duration);
        }

        self.last_time = Some(instant);
    }
}

impl Deref for TimeHistogram {
    type Target = Histogram;

    fn deref(&self) -> &Self::Target {
        &self.histogram
    }
}

impl Display for TimeHistogram {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::fmt::Result {
        self.histogram.fmt(fmt)
    }
}

/// A TimeWeightedSum is a wrapper around WeightedSum that keeps track of the last Instant a
/// value was observed, and uses the duration since that last observance to weight the added value.
#[derive(Clone, Copy, Debug, Default)]
pub struct TimeWeightedSum {
    sum: WeightedSum,
    last_observation: Option<Instant>,
}

impl TimeWeightedSum {
    pub fn add(&mut self, value: f64, instant: Instant) {
        if let Some(then) = self.last_observation {
            let duration = instant.saturating_duration_since(then).as_secs_f64();
            self.sum.add(value, duration);
        }

        self.last_observation = Some(instant);
    }
}

impl Deref for TimeWeightedSum {
    type Target = WeightedSum;

    fn deref(&self) -> &Self::Target {
        &self.sum
    }
}

impl Display for TimeWeightedSum {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::fmt::Result {
        self.sum.fmt(fmt)
    }
}