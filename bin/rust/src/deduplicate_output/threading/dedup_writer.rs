use anyhow::Result;
use crossbeam_channel::Receiver;
use std::fs;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;

use crate::deduplicate_output::io::{write_meta, write_sequences};
use crate::shared::BinEntryMeta;

pub fn spawn_writers(
    rx_out: Receiver<(String, Vec<BinEntryMeta>)>,
    outdir: &Path,
) -> std::thread::JoinHandle<Result<()>> {
    std::thread::spawn({
        let outdir = outdir.to_path_buf();

        move || -> Result<()> {
            fs::create_dir_all(&outdir)?;

            let seq_file = File::create(outdir.join("peptides.fasta"))?;
            let meta_file = File::create(outdir.join("metadata.csv"))?;

            let mut seq_writer = BufWriter::new(seq_file);
            let mut meta_writer = BufWriter::new(meta_file);

            writeln!(meta_writer, "ID,ACC,SPOS,EPOS,MSSCLVG,QUALIFIERS")?;

            for (id, (sequence, metas)) in rx_out.iter().enumerate() {
                write_sequences(&mut seq_writer, id, &sequence)?;
                write_meta(&mut meta_writer, id, &metas)?;
            }

            seq_writer.flush()?;
            meta_writer.flush()?;

            Ok(())
        }
    })
}
