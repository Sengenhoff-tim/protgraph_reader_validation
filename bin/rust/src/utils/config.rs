use anyhow::Result;
use clap::Parser;

use crate::utils::Cli;
use crate::traversal::{Interval, IntervalVecExt};
use crate::io::{read_query_csv};

const WEIGHT_FACTOR: i64 = 1000000000; //as per the original implementation

pub struct Config{
    pub cli: Cli,
    pub intervals: Vec<Interval>
}

impl Config {
    pub fn new() -> Result<Config> {
        let cli = Cli::parse();
    
        let intervals = read_query_csv(&cli.query_input_path, WEIGHT_FACTOR)?;
        
        let chunked = intervals.to_chunks((cli.interval_bin_size as i64) * WEIGHT_FACTOR)?; // cannot overflow

        Ok(Config {
            cli,
            intervals: chunked,
        })
    }
}
