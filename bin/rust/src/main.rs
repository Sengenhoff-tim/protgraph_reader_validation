use anyhow::{Result};
use clap::Parser;
use std::path::PathBuf;

mod io;
mod traversal;
mod workflows;

use crate::traversal::{IntervalVecExt};
use crate::io::{read_query_csv};
use crate::workflows::{process_graphs_to_fasta, process_graphs_dedublicated};

const WEIGHT_FACTOR: i64 = 1000000000; //as per the original implementation

#[derive(clap::Parser, Debug)]
struct Cli {
    #[arg(short = 'g', long = "graphs", value_name = "PATH", help = ".bpcsr output file from ProtGraph, containing protein graphs" )]
    graphs: PathBuf,

    #[arg(short = 'q', long = "queries", value_name = "PATH", help = ".csv file, containing queries" )]
    queries: PathBuf,

    #[arg(short = 'x', long = "max_vars", value_name = "U8", default_value_t = 3, help = "maximum divergences from reference for each fragment" )]
    max_vars: u8,

    #[arg(short = 'o', long = "output", value_name = "PATH", help = "output file name" )]
    output: PathBuf,

    #[arg(short = 'd', long = "dedublicate", value_name = "BOOL", default_value_t = true, help = "dedublicate output: will write an additional file to fasta" )]
    dedublicate: bool,

    #[arg(short = 't', long = "threads", value_name = "U8", default_value_t = 10 , help = "thread count" )]
    thread_count: u8,

    #[arg(short = 'i', long = "interval_bin_length", value_name = "I64", help = "length of interval bins" )]
    interval_bins: Option<i64>,
}



fn main() -> Result<()> {
    let cli = Cli::parse();
    
    let intervals = read_query_csv(&cli.queries, WEIGHT_FACTOR)?;

    let intervals = if let Some(bin_size) = cli.interval_bins {
        intervals.to_chunks(bin_size*WEIGHT_FACTOR)
    } else {
        intervals
    };

    if cli.dedublicate {
        process_graphs_dedublicated(
            cli.graphs, 
            &intervals, 
            cli.max_vars, 
            cli.thread_count as usize,
        )?;
        
    } else {
        process_graphs_to_fasta(
            cli.graphs, 
            cli.output,
            &intervals, 
            cli.max_vars, 
            cli.thread_count as usize,
        )?;
    }
        
    Ok(())
}
