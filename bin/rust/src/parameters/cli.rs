use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
pub struct Cli {
    #[arg(
        short = 'g',
        long = "graphs",
        value_name = "PATH",
        help = "Path to protein graph file (.bpcsr from ProtGraph)"
    )]
    pub graph_input_path: PathBuf,

    #[arg(
        short = 'q',
        long = "queries",
        value_name = "PATH",
        help = "Path to query CSV file. Format: 'lower,upper\\n600,700\\n...' (inclusive ranges in Da)"
    )]
    pub query_input_path: PathBuf,

    #[arg(
        short = 'o',
        long = "output",
        value_name = "PATH",
        help = "Output directory"
    )]
    pub output_path: PathBuf,

    #[arg(
        short = 'v',
        long = "max_vars",
        value_name = "U8",
        default_value_t = 3,
        help = "Maximum variants per peptide (default: 3; higher values increase computation)"
    )]
    pub max_vars: u8,

    #[arg(
        short = 'p',
        long = "avail_processors",
        value_name = "U8",
        default_value_t = 1,
        help = "Number of available processors"
    )]
    pub avail_processors: u8,

    #[arg(
        short = 'b',
        long = "hash_bits",
        value_name = "U8",
        help = "Creates 2^hash_bits intermediate bins as files. Only used when deduplicating. Gives rough control over intermediate file size. Defaults to auto-tuned value based on max file handles."
    )]
    pub hash_bits: Option<u8>,

    #[arg(
        short = 'h',
        long = "max_file_handles",
        value_name = "U8",
        help = "Maximum file handles for intermediate files. Only used when deduplicating. With Unix, defaults to RLIMIT_NOFILE. With Windows, defaults to 2048"
    )]
    pub max_handles: Option<u32>,

    #[arg(
        short = 'm',
        long = "avail_memory",
        value_name = "U8",
        help = "Available memory in GB. When estimated usage exceeds this, jobs are split and rescheduled. Splitting should be avoided. See documentation for details."
    )]
    pub avail_memory: u8,

    #[arg(
        short = 'i',
        long = "interval_bin_length",
        value_name = "U16",
        default_value_t = 100,
        help = "Interval bin size in Da. Smaller bins = finer-grained memory estimation and more frequent job splits"
    )]
    pub interval_bin_size: u16,

    #[arg(
        short = 's',
        long = "job_splits",
        value_name = "U8",
        default_value_t = 4,
        help = "Number of sub-jobs created per split when memory limit reached"
    )]
    pub job_splits: u8,

    #[arg(
        short = 'd',
        long = "split_depth",
        value_name = "U8",
        default_value_t = 5,
        help = "Maximum times a job can be recursively split (increases exponentially; failed jobs saved to failed.csv)"
    )]
    pub job_split_depth: u8,
}
