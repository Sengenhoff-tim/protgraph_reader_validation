use std::{fs::File, path::PathBuf};

use anyhow::{Result, bail};
use csv::Reader;

const MIN_BOUND: i64 = 0;
const MAX_BOUND: i64 = 50000; // theoretical max before overflow: 9_223_372_036

use crate::process_graphs::utilities::Interval;

pub fn read_query_csv(path: &PathBuf, weight_factor: i64) -> Result<Vec<Interval>> {
    let file = File::open(path)?;
    let mut reader = Reader::from_reader(file);
    
    let intervals: Vec<Interval> = reader
        .deserialize()
        .map(|result| {
            let mut interval: Interval = result?;

            validate_interval(&interval)?;

            interval.lower *= weight_factor;
            interval.upper *= weight_factor;
            Ok(interval)
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(intervals)
}

fn validate_interval(interval: &Interval) -> Result<()> {
    if interval.lower < MIN_BOUND || interval.lower > MAX_BOUND {
        bail!(
            "Invalid lower bound: {} (expected {}-{})",
            interval.lower,
            MIN_BOUND,
            MAX_BOUND
        );
    }
    if interval.upper < MIN_BOUND || interval.upper > MAX_BOUND {
        bail!(
            "Invalid upper bound: {} (expected {}-{})",
            interval.upper,
            MIN_BOUND,
            MAX_BOUND
        );
    }
    if interval.lower > interval.upper {
        bail!(
            "Invalid interval: lower ({}) > upper ({})",
            interval.lower,
            interval.upper
        );
    }
    Ok(())
}

