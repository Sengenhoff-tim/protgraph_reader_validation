use anyhow::Result;
use clap::Parser;

use crate::parameters::Cli;
use crate::parameters::io::read_query_csv::read_query_csv;
use crate::process_graphs::utilities::{Interval, IntervalVecExt};

const WEIGHT_FACTOR: i64 = 1000000000; //as per the original implementation

/// struct containing the collated input
pub struct Config {
    pub cli: Cli,
    pub intervals: Vec<Interval>,
}

impl Config {
    pub fn new() -> Result<Config> {
        let cli = Cli::parse();

        // queries are read from query input csv in Da
        let intervals = read_query_csv(&cli.query_input_path, WEIGHT_FACTOR)?;

        // intervals are split into bins and converted for internal representation
        let chunked = intervals.to_chunks((cli.interval_bin_size as i64) * WEIGHT_FACTOR)?; // cannot overflow

        Ok(Config {
            cli,
            intervals: chunked,
        })
    }
}
