
use crossbeam_channel::{bounded, Receiver, Sender};
use std::fs;
use std::io::{Write};
use std::{collections::HashMap, path::PathBuf};
use anyhow::{Context, Result};
use bincode::config::standard;
use std::fs::File;
use std::io::{BufReader, ErrorKind, Read};
use std::path::Path;

use crate::traversal::{Entry, EntryMeta};
use crate::io::tmp_files::WriterManagerResult;

fn process_shard(
    entries: Vec<Entry>,
    tx_out: &Sender<(String, Vec<EntryMeta>)>,
) -> Result<()> {

    let mut unique_entries: HashMap<String, Vec<EntryMeta>> = HashMap::new();

    for e in entries {
        unique_entries
            .entry(e.pep)
            .or_insert_with(Vec::new)
            .push(e.meta);
    }

    for (key, metas) in unique_entries {
        tx_out.send((key, metas))?;
    }

    

    Ok(())
}

fn spawn_dispatcher(
    result: WriterManagerResult,
    tx: Sender<Vec<Entry>>,
) -> std::thread::JoinHandle<Result<()>> {
    std::thread::spawn(move || -> Result<()> {
        for path in result.filenames {
            let entries = read_entries_binary(&path)
                .with_context(|| {
                    format!(
                        "failed to read entries from {}",
                        path.display()
                    )
                })?;

            tx.send(entries)
                .context("failed to send decoded entries")?;
        }

        Ok(())
    })
}


fn spawn_worker(
    rx: Receiver<Vec<Entry>>,
    tx_out: Sender<(String, Vec<EntryMeta>)>,
) -> std::thread::JoinHandle<Result<()>> {
    std::thread::spawn(move || -> Result<()> {
        while let Ok(shard) = rx.recv() {
            process_shard(shard, &tx_out)?;
        }

        Ok(())
    })
}

pub fn bin_reader_manager(
    result: WriterManagerResult,
    num_threads: usize,
    outdir: PathBuf
) -> Result<()> {
    let (tx_in, rx_in) = bounded::<Vec<Entry>>(2);

    let (tx_out, rx_out) = bounded::<(String, Vec<EntryMeta>)>(100);

    let writer_handle = spawn_writers(rx_out, outdir);

    let mut worker_handles = Vec::new();
    
    for _ in 0..num_threads {
        let h = spawn_worker(rx_in.clone(), tx_out.clone());
        worker_handles.push(h);
    }

    drop(tx_out);

    let dispatcher_handles = spawn_dispatcher(result, tx_in);

    dispatcher_handles
        .join()
        .map_err(|_| anyhow::anyhow!("dispatcher panicked"))??;

    for h in worker_handles {
        h.join().map_err(|_| anyhow::anyhow!("Worker thread panicked"))??;
    }
    
    writer_handle.join().map_err(|_| anyhow::anyhow!("Writer thread panicked"))??;
    

    Ok(())
}

fn spawn_writers(
    rx_out: Receiver<(String, Vec<EntryMeta>)>,
    outdir: PathBuf
) -> std::thread::JoinHandle<Result<()>> {
    std::thread::spawn(move || -> Result<()> {
        fs::create_dir_all(&outdir)?;
        let seq_file = std::fs::File::create(&outdir.join("peptides.fasta"))?;
        let meta_file = std::fs::File::create(&outdir.join("metadata.csv"))?;

        let mut seq_writer = std::io::BufWriter::new(seq_file);
        let mut meta_writer = std::io::BufWriter::new(meta_file);

        writeln!(meta_writer, "ID,ACC,SPOS,EPOS,MSSCLVG,QUALIFIERS")?;

        for (id, (sequence, metas)) in rx_out.iter().enumerate() {
            write_sequences(&mut seq_writer, id as u128, &sequence)?;
            write_meta(&mut meta_writer, id as u128, &metas)?;
        }

        seq_writer.flush()?;
        meta_writer.flush()?;
        
        Ok(())
    })
}


fn write_sequences(
    writer: &mut std::io::BufWriter<std::fs::File>,
    id: u128,
    sequence: &str,
) -> Result<()> {
    writeln!(
        writer,
        ">pg|{}\n{}",
        id,
        sequence
    )?;
    Ok(())
}

fn write_meta(
    writer: &mut std::io::BufWriter<std::fs::File>,
    id: u128,
    metas: &[EntryMeta],
) -> Result<()>{
    for meta in metas{
        let spos = meta.spos.map(|v| v.to_string()).unwrap_or_else(|| "?".to_string());
        let epos = meta.epos.map(|v| v.to_string()).unwrap_or_else(|| "?".to_string());
        let qualifiers = meta.qualifiers.replace(",", "|");
        writeln!(
        writer,
        "{},{},{},{},{},[{}]",
        id,
        meta.acc,
        spos,
        epos,
        meta.mssclvg,
        qualifiers
    )?;
    }
    Ok(())

}



pub fn read_entries_binary(
    path: impl AsRef<Path>,
) -> Result<Vec<Entry>> {
    let path = path.as_ref();

    let file = File::open(path)
        .with_context(|| format!("failed to open {}", path.display()))?;

    let mut reader = BufReader::new(file);

    let mut entries = Vec::new();

    loop {
        let mut len_buf = [0u8; 4];

        match reader.read_exact(&mut len_buf) {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                break;
            }
            Err(e) => {
                return Err(e)
                    .with_context(|| format!(
                        "failed reading length prefix from {}",
                        path.display()
                    ));
            }
        }

        let len = u32::from_le_bytes(len_buf) as usize;

        let mut bytes = vec![0u8; len];

        reader
            .read_exact(&mut bytes)
            .with_context(|| format!(
                "failed reading {} bytes from {}",
                len,
                path.display()
            ))?;

        let (entry, _): (Entry, usize) =
            bincode::decode_from_slice(
                &bytes,
                standard(),
            )
            .with_context(|| format!(
                "failed to deserialize entry from {}",
                path.display()
            ))?;

        entries.push(entry);
    }

    Ok(entries)
}