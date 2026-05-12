use anyhow::{Context, Result};
use crossbeam_channel::{Receiver, Sender};
use lru::LruCache;
use std::{
    collections::HashMap,
    fs::{create_dir_all, File, OpenOptions},
    io::{BufWriter, Write},
    num::NonZeroUsize,
    path::{Path, PathBuf},
    thread::{self, JoinHandle},
};
use bincode::{encode_to_vec, config::standard};

use crate::traversal::Entry;

/* 
let bits = required_hash_bits(
    500_000_000,             // entries
    80,                      // avg serialized size
    2 * 1024 * 1024 * 1024, // max_memory
    2.5,                     // in-memory overhead
    2.0,                     // skew factor
);
*/

fn required_hash_bits(
    writer_params: &BinWriterParams
) -> u32 {
    let total_bytes =
        writer_params.total_entries as f64
        * writer_params.avg_entry_size as f64
        * writer_params.overhead
        * writer_params.skew;

    let required_shards =
        (total_bytes / writer_params.max_memory as f64).ceil() as u64;

    required_shards
        .max(1)
        .next_power_of_two()
        .trailing_zeros()
}
pub struct WriterManagerResult {
    pub filenames: Vec<PathBuf>,
    pub handles: LruCache<PathBuf, BufWriter<File>>,
}

///
/// Dumb binary writer:
///
/// [u32 record_len LE]
/// [bincode payload]
///
fn write_entry_binary(
    writer: &mut BufWriter<File>,
    entry: &Entry,
) -> Result<()> {
    let bytes = encode_to_vec(
        entry,
        standard(),
    )?;

    let len = bytes.len() as u32;

    writer.write_all(&len.to_le_bytes())?;
    writer.write_all(&bytes)?;

    Ok(())
}

fn open_writer(path: &Path) -> Result<BufWriter<File>> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .read(true)
        .write(true)
        .open(path)?;

    Ok(BufWriter::new(file))
}

fn shard_filename(
    out_dir: &Path,
    shard_id: usize,
) -> PathBuf {
    out_dir.join(format!("shard_{shard_id:05}.bin"))
}

pub struct BinWriterParams{
    pub total_entries: u64,
    pub avg_entry_size: u64,
    pub max_memory: u64,
    pub overhead: f64,
    pub skew: f64,
    pub entry_channel_size: u64
}
///
/// Spawn writer manager thread.
///
/// Returns:
/// - Sender<(u64, Entry)>
/// - JoinHandle<Result<WriterManagerResult>>
///
pub fn spawn_writer_manager(
    out_dir: impl AsRef<Path>,
    params: BinWriterParams,
    max_open_files: usize,
) -> Result<(
    Sender<(u64, Entry)>,
    JoinHandle<Result<WriterManagerResult>>,
)> {
    let out_dir = out_dir.as_ref().to_path_buf();

    create_dir_all(&out_dir)
        .with_context(|| format!("failed to create {}", out_dir.display()))?;

    let hash_bits = required_hash_bits(&params);

    let (tx, rx) = crossbeam_channel::bounded::<(u64, Entry)>(params.entry_channel_size as usize);

    let handle = thread::spawn(move || {
        writer_manager_thread(
            rx,
            out_dir,
            hash_bits,
            max_open_files,
        )
    });

    Ok((tx, handle))
}

fn writer_manager_thread(
    rx: Receiver<(u64, Entry)>,
    out_dir: PathBuf,
    hash_bits: u32,
    max_open_files: usize,
) -> Result<WriterManagerResult> {
    
    let shard_mask = (1usize << hash_bits) - 1;

    // shard_id -> filename
    let mut filenames: HashMap<usize, PathBuf> = HashMap::new();

    // filename -> writer
    let mut writers: LruCache<PathBuf, BufWriter<File>> =
        LruCache::new(
            NonZeroUsize::new(max_open_files)
                .context("max_open_files must be > 0")?,
        );

    while let Ok(entry) = rx.recv() {
        let shard_id =
            (entry.0 as usize) & shard_mask;

        let path = filenames
            .entry(shard_id)
            .or_insert_with(|| shard_filename(&out_dir, shard_id))
            .clone();

        // open writer if absent
        if !writers.contains(&path) {
            let writer = open_writer(&path)?;

            // evicted handles get flushed before drop
            if let Some((_, mut evicted)) =
                writers.push(path.clone(), writer)
            {
                evicted.flush()?;
            }
        }

        let writer = writers
            .get_mut(&path)
            .context("writer disappeared unexpectedly")?;

        write_entry_binary(writer, &entry.1)?;
    }

    // flush remaining handles
    for (_, writer) in writers.iter_mut() {
        writer.flush()?;
    }

    let mut files: Vec<PathBuf> =
        filenames.into_values().collect();

    files.sort();

    Ok(WriterManagerResult {
        filenames: files,
        handles: writers,
    })
}