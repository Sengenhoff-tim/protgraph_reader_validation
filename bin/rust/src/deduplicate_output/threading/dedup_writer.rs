use std::{
    fs,
    fs::File,
    io::{Write, BufWriter},
    path::Path
};

use flate2::{Compression, write::GzEncoder};
use anyhow::Result;
use crossbeam_channel::Receiver;

use crate::deduplicate_output::io::{write_meta, write_sequences};
use crate::shared::BinEntryMeta;


const OUT_FASTA_FILE: &str = "peptides.fasta";
const OUT_METADATA_FILE: &str = "metadata.csv";

const META_HEADER_LINE: &str = "ID,ACC,SPOS,EPOS,MSSCLVG,QUALIFIERS";

pub fn spawn_writers(
    rx_out: Receiver<(String, Vec<BinEntryMeta>)>,
    outdir: &Path,
) -> std::thread::JoinHandle<Result<()>> {
    std::thread::spawn({
        let outdir = outdir.to_path_buf();

        move || -> Result<()> {
            fs::create_dir_all(&outdir)?;

            let seq_file = File::create(outdir.join(OUT_FASTA_FILE))?;
            let meta_file = File::create(outdir.join(OUT_METADATA_FILE))?;

            let seq_encoder = GzEncoder::new(seq_file, Compression::default());
            let meta_encoder = GzEncoder::new(meta_file, Compression::default());

            let mut seq_writer = BufWriter::new(seq_encoder);
            let mut meta_writer = BufWriter::new(meta_encoder);

            writeln!(meta_writer, "{}", META_HEADER_LINE)?;

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
