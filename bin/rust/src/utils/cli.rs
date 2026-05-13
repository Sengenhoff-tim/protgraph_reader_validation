use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
pub struct Cli {
    #[arg(short = 'g', long = "graphs", value_name = "PATH", help = ".bpcsr output file from ProtGraph, containing protein graphs" )]
    pub graphs: PathBuf,

    #[arg(short = 'q', long = "queries", value_name = "PATH", help = ".csv file, containing queries" )]
    pub queries: PathBuf,

    #[arg(short = 'x', long = "max_vars", value_name = "U8", default_value_t = 3, help = "maximum divergences from reference for each fragment" )]
    pub max_vars: u8,

    #[arg(short = 'o', long = "output", value_name = "PATH", help = "output file name" )]
    pub output: PathBuf,

    #[arg(short = 'd', long = "dedublicate", help = "dedublicate output: will write an additional file to fasta" )]
    pub dedublicate: bool,

    #[arg(short = 't', long = "threads", value_name = "U8", default_value_t = 10 , help = "thread count" )]
    pub thread_count: u8,

    #[arg(short = 'i', long = "interval_bin_length", value_name = "I64", help = "length of interval bins" )]
    pub interval_bins: Option<i64>,
}