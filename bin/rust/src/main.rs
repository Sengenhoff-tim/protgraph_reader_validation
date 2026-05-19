use anyhow::Result;

mod deduplicate_output;
mod parameters;
mod process_graphs;
mod shared;

use crate::{
    deduplicate_output::dedup_bin_files, parameters::Config, process_graphs::process_graphs,
};

/// Reads BPCSR files produced by ProtGraph and generates:
/// - a deduplicated `peptides.fasta`
/// - `metadata.csv`, describing which proteins generated each peptide
/// - `log.csv`, containing run information
fn main() -> Result<()> {
    let config = Config::new()?;

    let avail_processors = config.cli.avail_processors as usize;
    let output_path = config.cli.output_path.clone();

    // read graphs and produce intermediate files
    let tmp_files = process_graphs(config)?;

    // read intermediate files and write deduplicated output
    dedup_bin_files(tmp_files, avail_processors, &output_path)?;

    Ok(())
}
