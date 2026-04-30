use anyhow::{Result, anyhow};
use clap::Parser;
use rayon::prelude::*;
use std::sync::Arc;
use std::sync::Mutex;
use crossbeam_channel::{bounded, Receiver, Sender};
use std::fs::File;
use std::io::{BufReader};
use std::thread;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

mod protgraph_io;
mod protgraph_types;

use crate::protgraph_types::{ProteinGraph, Interval};
use crate::protgraph_io::{ProteinGraphReader, fasta_writer_thread, read_query_csv};

const WEIGHT_FACTOR: i64 = 1000000000; //as per the original implementation

#[derive(clap::Parser, Debug)]
struct Cli {
    #[arg(short = 'g', long = "graphs", value_name = "GRAPHS", help = ".bpcsr output file from ProtGraph, containing protein graphs" )]
    graphs: PathBuf,

    #[arg(short = 'q', long = "queries", value_name = "QUERIES", help = ".csv file, containing queries" )]
    queries: PathBuf,

    #[arg(short = 'x', long = "max_vars", value_name = "MAXVARS", help = "maximum divergences from reference for each fragment" )]
    max_vars: u8,

    #[arg(short = 'o', long = "output", value_name = "OUTPUT", help = "output file name" )]
    output: PathBuf,
}

fn process_graphs(
    graph_input_path: PathBuf,
    output_path: PathBuf, 
    intervals: Vec<Interval>, 
    max_vars: u8, 
    trace: Arc<Mutex<BufWriter<File>>>, 
) -> anyhow::Result<()> {

    let file = File::open(&graph_input_path)?;
    let reader = ProteinGraphReader::new(
        BufReader::new(file)
    );

    for graph in reader {
        match graph {
            Ok(protein_graph) => { 
                let protein_graph = std::sync::Arc::new(protein_graph);
                // Using compact format
                protein_graph.write_to_file_compact("intervals_compact_rust_.txt")?;
                process_single_graph(protein_graph, &intervals, max_vars, output_path.clone(), trace.clone())?; 
            }
            Err(e) => {
                eprintln!("error reading graph: {}", e);
                break;
            }
        }
        
    }
    Ok(())
}

fn process_single_graph( 
    graph: Arc<ProteinGraph>, 
    intervals: &[Interval], 
    max_vars: u8, 
    output_path: PathBuf, 
    trace: Arc<Mutex<BufWriter<File>>>
) -> anyhow::Result<()> { 
    let (tx, rx): (
        Sender<Vec<Vec<u32>>>, 
        Receiver<Vec<Vec<u32>>>
    ) = bounded(128); 
    let graph_for_writer = Arc::clone(&graph); 
    let writer_handle = thread::spawn(move || { 
        fasta_writer_thread(
            rx, 
            output_path, 
            graph_for_writer
        ) 
    }); 
    
    spawn_producers(
        graph, 
        intervals, 
        max_vars, 
        tx,
        trace.clone()
    )?; 
    
    writer_handle
        .join()
        .map_err(|_| anyhow!("Writer thread panicked"))??; 
    
    Ok(()) 
}
/*
fn process_single_graph(
    graph: Arc<ProteinGraph>,
    intervals: &[Interval],
    max_vars: u8,
    output_path: PathBuf,
) -> anyhow::Result<()> {
    let (
        tx, 
        rx
    ): (
        Sender<Vec<u32>>,
        Receiver<Vec<u32>>
    ) = bounded(128);

    let graph_for_writer = Arc::clone(&graph);

    let writer_handle = thread::spawn(move || {
        fasta_writer_thread(rx, output_path, graph_for_writer)
    });

    spawn_producers(graph, intervals, max_vars, tx)?;

    writer_handle.join().map_err(|_| anyhow!("Writer thread panicked"))??;

    Ok(())
}
fn spawn_producers(
    graph: Arc<ProteinGraph>,
    intervals: &[Interval],
    max_vars: u8,
    tx: Sender<Vec<u32>>,
) -> Result<()> {
    intervals.par_iter().for_each(|interval| {
        let result: Result<()> = (|| {
            graph.traverse_varcount_streaming(interval, max_vars, |path| {
                tx.send(path.to_vec())
                    .map_err(|e| anyhow!("send failed: {}", e))?;
                Ok(())
            })?;
            Ok(())
        })();

        if let Err(e) = result {
            eprintln!("interval {:?} failed: {:?}", interval, e);
        }
    });

    drop(tx);
    Ok(())
}

fn spawn_producers(
    graph: Arc<ProteinGraph>,
    intervals: &[Interval], 
    max_vars: u8, 
    tx: Sender<Vec<Vec<u32>>>, 
) -> anyhow::Result<()> {
    intervals.par_iter().try_for_each(|interval| {
        let paths = graph.traverse_varcount(interval, max_vars)?; 
        tx.send(paths).map_err(|e| anyhow!(e))?; 
        Ok::<(), Error>(()) 
    })?; 

    drop(tx); 
    
    Ok(()) 
}
*/

fn spawn_producers(
    graph: Arc<ProteinGraph>,
    intervals: &[Interval],
    max_vars: u8,
    tx: Sender<Vec<Vec<u32>>>,
    trace: Arc<Mutex<BufWriter<File>>>
) -> Result<()> {
    intervals.par_iter().for_each(|interval| {
        let result: Result<()> = (|| {
            let paths = graph
                .traverse_varcount(interval, max_vars, trace.clone())
                .map_err(|e| anyhow!("traversal failed: {e}"))?;

            tx.send(paths)
                .map_err(|e| anyhow!("channel send failed: {e}"))?;

            Ok(())
        })();

        if let Err(e) = result {
            eprintln!("interval failed: {e:?}");
        }
    });

    drop(tx);

    Ok(())
}



fn main() -> Result<()> {
    let trace_file = File::create("rust_trace.log")?;
    let trace = Arc::new(Mutex::new(BufWriter::new(trace_file)));
    let cli = Cli::parse();
    
    let intervals = read_query_csv(&cli.queries, WEIGHT_FACTOR)?;

let file = File::create("intervals_out.csv")?;
let mut writer = BufWriter::new(file);

// Optional header
writeln!(writer, "lower,upper")?;

for iv in &intervals {
    writeln!(writer, "{},{}", iv.lower, iv.upper)?;
}
    /*
    write_dummy_fasta("dummy.fasta".into())?;
    */
    process_graphs(cli.graphs, cli.output, intervals, cli.max_vars, trace)?;
    Ok(())
}
