use anyhow::Result;
use csv::Reader;
use std::fs::File;
use std::path::PathBuf;

use crate::traversal::{Interval};

pub fn read_query_csv(path: &PathBuf, weight_factor: i64) -> Result<Vec<Interval>> {
    let file = File::open(path)?;
    let mut reader = Reader::from_reader(file);
    
    let intervals: Vec<Interval> = reader
        .deserialize()
        .map(|result| {
            let mut interval: Interval = result?;
            interval.lower *= weight_factor;
            interval.upper *= weight_factor;
            Ok(interval)
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(intervals)
}
