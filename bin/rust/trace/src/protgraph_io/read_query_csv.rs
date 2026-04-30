use anyhow::Result;
use csv::Reader;
use std::fs::File;
use std::path::PathBuf;

use crate::protgraph_types::{Interval};//, Raw64};
/* 
use serde::Deserialize;

#[derive(Deserialize)]
struct IntervalCsv {
    lower: u64,
    upper: u64,
}

pub fn read_query_csv(path: &PathBuf, weight_factor: i64) -> Result<Vec<Interval>> {
    let file = File::open(path)?;
    let mut reader = csv::Reader::from_reader(file);

    let mut out = Vec::new();

    for result in reader.deserialize::<IntervalCsv>() {
        let rec = result?;

        // convert factor into u64-safe scaling
        let factor = weight_factor as u64;

        let lower = Raw64(rec.lower.wrapping_mul(factor));
        let upper = Raw64(rec.upper.wrapping_mul(factor));

        out.push(Interval { lower, upper });
    }

    Ok(out)
}
*/

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
