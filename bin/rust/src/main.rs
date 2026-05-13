use anyhow::{Result, anyhow};
use clap::Parser;

mod io;
mod utils;
mod traversal;
mod workflows;

use crate::utils::Cli;
use crate::traversal::{IntervalVecExt};
use crate::io::{read_query_csv};
use crate::workflows::{process_graphs_dedublicated};


const WEIGHT_FACTOR: i64 = 1000000000; //as per the original implementation

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    let intervals = read_query_csv(&cli.queries, WEIGHT_FACTOR)?;

    let intervals = if let Some(bin_size) = cli.interval_bins {
        let scaled_chunk_size = bin_size
            .checked_mul(WEIGHT_FACTOR)
            .ok_or_else(|| {
                anyhow!(
                    "chunk size overflow: {} * {}",
                    bin_size,
                    WEIGHT_FACTOR
                )
            })?;

        intervals.to_chunks(scaled_chunk_size)?
    } else {
        intervals
    };

    if cli.dedublicate {
        process_graphs_dedublicated(
            cli.graphs, 
            cli.output,
            intervals, 
            cli.max_vars, 
            cli.thread_count as usize,
        )?;
    } else {
        return Ok(());
    }
        
    Ok(())
}
