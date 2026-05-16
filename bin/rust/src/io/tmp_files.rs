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
use xxhash_rust::xxh64::xxh64;

use crate::traversal::Entry;

const SEED: u64 = 0xC0111DE;

#[cfg(unix)]
fn get_sys_open_files() -> u32 {
    use nix::sys::resource::{getrlimit, Resource};

    match getrlimit(Resource::RLIMIT_NOFILE) {
        Ok((soft, _)) => {
            (soft / 2).clamp(64, 8192) as u32
        }
        Err(_) => 512,
    }
}

#[cfg(windows)]
fn get_sys_open_files() -> u32 {
    2048
}

fn hash_bits_for(target_shards: u32) -> u8 {

    let shards = target_shards.next_power_of_two();

    let bits = (usize::BITS - (shards - 1).leading_zeros()) as u8;

    bits
}

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

pub fn spawn_writer_manager(
    out_dir: impl AsRef<Path>,
    hash_bits: Option<u8>,
    max_handles: Option<u32>,
    avail_processors: u8
) -> Result<(
    Sender<Entry>,
    JoinHandle<Result<Vec<PathBuf>>>,
)> {
    let out_dir = out_dir.as_ref().to_path_buf();

    create_dir_all(&out_dir)
        .with_context(|| format!("failed to create {}", out_dir.display()))?;


    let (tx, rx) = crossbeam_channel::bounded::<Entry>((avail_processors*2) as usize);

    let handle = thread::spawn(move || {
        writer_manager_thread(
            rx,
            out_dir,
            hash_bits,
            max_handles,
        )
    });

    Ok((tx, handle))
}

fn shard_filename(
    out_dir: &Path,
    hash: u64,
    shard_id: usize,
    use_subdirs: bool,
) -> PathBuf {
    let filename = format!("{shard_id:05x}.bin");

    if use_subdirs {
        let dir = format!("{:02x}", (hash >> 56) & 0xff);

        out_dir.join(dir).join(filename)
    } else {
        out_dir.join(filename)
    }
}

fn writer_manager_thread(
    rx: Receiver<Entry>,
    out_dir: PathBuf,
    hash_bits: Option<u8>,
    max_handles: Option<u32>,
) -> Result<Vec<PathBuf>> {

    // determine maximum file handles if not set
    let max_h = max_handles.unwrap_or_else(|| {
        (get_sys_open_files() * 7) / 10
    });

    // determine hash bits if not set
    let h_bits = hash_bits.unwrap_or_else(|| {
        hash_bits_for(max_h / 2)
    });

    let shard_mask = (1usize << h_bits) - 1;

    // enable directory fanout once 256 shards are surpassed
    let use_subdirs = h_bits > 8;

    // lru for file handles
    let mut writers: LruCache<PathBuf, BufWriter<File>> =
        LruCache::new(
            NonZeroUsize::new(max_h as usize)
                .context("max_open_files must be > 0")?,
        );

    // shard_id -> filename
    let mut filenames: HashMap<usize, PathBuf> = HashMap::new();

    let tmp_path = &out_dir.join("tmp");

    while let Ok(entry) = rx.recv() {
        let path = resolve_path(
            &entry,
            &tmp_path,
            shard_mask,
            use_subdirs,
            &mut filenames,
        );

        ensure_parent_dir(&path)?;

        let writer = get_writer(&mut writers, &path)?;

        write_entry_binary(writer, &entry)?;
    }

    // flush remaining handles
    for (_, writer) in writers.iter_mut() {
        writer.flush()?;
    }

    let mut files: Vec<PathBuf> =
        filenames.into_values().collect();

    files.sort();

    Ok(files)
}

fn resolve_path(
    entry: &Entry,
    out_dir: &Path,
    shard_mask: usize,
    use_subdirs: bool,
    filenames: &mut HashMap<usize, PathBuf>,
) -> PathBuf {
    let hash = xxh64(entry.pep.as_bytes(), SEED);
    let shard_id = (hash as usize) & shard_mask;

    filenames
        .entry(shard_id)
        .or_insert_with(|| {
            shard_filename(
                out_dir,
                hash,
                shard_id,
                use_subdirs,
            )
        })
        .clone()
}

fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn get_writer<'a>(
    writers: &'a mut LruCache<PathBuf, BufWriter<File>>,
    path: &PathBuf,
) -> Result<&'a mut BufWriter<File>> {
    if !writers.contains(path) {
        let writer = open_writer(path)?;

        if let Some((_, mut evicted)) = writers.push(path.clone(), writer) {
            evicted.flush()?;
        }
    }

    writers
        .get_mut(path)
        .context("writer disappeared unexpectedly")
}