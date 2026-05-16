use anyhow::{Result, anyhow};
use bincode::config;
use clap::Parser;

mod io;
mod utils;
mod traversal;
mod workflows;

use crate::{utils::Config, workflows::process_graphs_deduplicated};

fn main() -> Result<()> {

    let config = Config::new()?;

    if config.cli.deduplicate {
        process_graphs_deduplicated(config)?;
    } else {
        return Ok(());
    }
        
    Ok(())
}
