use std::{
    fs::{create_dir_all, File},
    io::{BufReader, BufWriter},
    path::PathBuf,
    sync::Arc,
    thread,
};

use anyhow::{Context, Result};
use crossbeam_channel::bounded;

use crate::parameters::Config;
use crate::process_graphs::{
    graph::ProteinGraph,
    threading::{spawn_graph_dispatcher, spawn_protein_graph_reader, spawn_writer_manager},
};

const GB: u64 = 1024*1024*1024;
const LOG_FILE_NAME: &str = "logs.csv";

/// main graph processing workflow
pub fn process_graphs(
    config: Config
) -> Result<Vec<PathBuf>> {

    // create output directory
    let cli = &config.cli;
    let out_dir = &cli.output_path;
    create_dir_all(&out_dir)
        .with_context(|| format!("failed to create {}", out_dir.display()))?;

    // set up log writer
    let logs = File::create(&cli.output_path.join(LOG_FILE_NAME))?;
    let log_writer = BufWriter::new(logs);

    // setup graph reader 
    let graph = File::open(&cli.graph_input_path)?;
    let reader_for_graph = BufReader::new(graph);
    let (tx_graph, rx_graph) = bounded::<Result<ProteinGraph>>(2);
    let reader_handle = thread::spawn(|| spawn_protein_graph_reader(reader_for_graph, tx_graph));

    // spawn tmp file writer
    let (tx_entry, bin_writer_handle) = spawn_writer_manager(
        &cli.output_path,
        cli.hash_bits,
        cli.max_handles,
        cli.avail_processors
    )?;

    //process graphs
    let intervals = Arc::new(config.intervals);

    let graph_handle = spawn_graph_dispatcher(
        rx_graph,
        tx_entry,
        intervals,
        cli.max_vars,
        cli.avail_processors as usize,
        (cli.avail_memory as u64 *GB) as usize,
        cli.job_splits,
        cli.job_split_depth,
        log_writer
    )?;

    graph_handle
        .join()
        .map_err(|e| anyhow::anyhow!("thread panicked: {:?}", e))??;
    
    reader_handle.join().unwrap();

    let result = bin_writer_handle.join().unwrap()?;



    Ok(result)
}

