use anyhow::{Result, anyhow};
use crossbeam_channel::bounded;
use std::path::{Path, PathBuf};

use crate::deduplicate_output::threading::{spawn_dispatcher, spawn_worker, spawn_writers};
use crate::shared::{BinEntry, BinEntryMeta};

const CHANNEL_CAPACITY_IN: usize = 2;
const CHANNEL_CAPACITY_OUT_BASE: usize = 10;

pub fn dedup_bin_files(result: Vec<PathBuf>, num_threads: usize, outdir: &Path) -> Result<()> {
    let (tx_in, rx_in) = bounded::<Vec<BinEntry>>(CHANNEL_CAPACITY_IN);
    let (tx_out, rx_out) =
        bounded::<(String, Vec<BinEntryMeta>)>(CHANNEL_CAPACITY_OUT_BASE.min(num_threads * 2));

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
        .map_err(|_| anyhow!("dispatcher panicked"))??;

    for h in worker_handles {
        h.join().map_err(|_| anyhow!("Worker thread panicked"))??;
    }

    writer_handle
        .join()
        .map_err(|_| anyhow!("Writer thread panicked"))??;

    Ok(())
}
