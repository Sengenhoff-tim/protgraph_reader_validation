use std::io::{BufWriter, Write};
use std::fs::{OpenOptions};
use std::path::PathBuf;
use std::sync::Arc;
use anyhow::Result;
use crossbeam_channel::Receiver;

use crate::protgraph_types::ProteinGraph;

/*
pub fn fasta_writer_thread(
    rx: Receiver<Vec<u32>>,
    output_path: PathBuf,
    graph: Arc<ProteinGraph>,
) -> Result<()> {
    let output_path = if output_path.is_absolute() {
        output_path
    } else {
        std::env::current_dir()?.join(output_path)
    };

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(output_path)?;
    
    let mut writer = BufWriter::new(file);

    for path in rx {
        graph.write_fragment_to_fasta(&mut writer, &path)?;
    }

    writer.flush()?;

    Ok(())
}*/

pub fn fasta_writer_thread( 
    rx: Receiver<Vec<Vec<u32>>>, 
    output_path: PathBuf, 
    graph: Arc<ProteinGraph>, 
) -> Result<()> { 
    let output_path = if output_path.is_absolute() { 
        output_path 
    } 
    else { 
        std::env::current_dir()?.join(output_path) 
    }; 
    
    if let Some(parent) = output_path.parent() { 
        std::fs::create_dir_all(parent)?; 
    } 
    
    let file = OpenOptions::new() 
        .create(true)
        .append(true) 
        .open(output_path)?; 
    
    let mut writer = BufWriter::new(file); 
    
    for batch in rx { 
        for path in batch { 
            graph.write_fragment_to_fasta(&mut writer, &path)?; 
        } 
    } 
    
    writer.flush()?;
    drop(writer);
    
    Ok(()) 

}