use std::io::{Write};
use crossbeam_channel::Receiver;

use crate::traversal::Entry;

/* 
pub fn writer_thread<W: Write>(
    rx: Receiver<Vec<(u32, u32)>>,
    writer: &mut W,
    meta_data: Arc<MetaData>,
    sequences: Arc<StringTable>,
) -> Result<()> {
    for trace in rx {
        meta_data.write_fragment(sequences.clone(), writer, &trace)?;
    }

    writer.flush()?;
    Ok(())
}

    */

pub fn writer_thread<W: Write>(
    rx_entry: Receiver<Entry>,
    writer: &mut W,
) -> anyhow::Result<()> {
    //, 
    for entry in rx_entry {
        let spos = entry.spos.map(|v| v.to_string()).unwrap_or_else(|| "?".to_string());
        let epos = entry.epos.map(|v| v.to_string()).unwrap_or_else(|| "?".to_string());
        writeln!(
            writer,
            ">pg|TODO|{}({}:{},mssclvg:{},{})\n{}",
            entry.acc,
            spos,
            epos,
            entry.mssclvg,
            entry.qualifiers,
            entry.pep
            )?;
    }

    writer.flush()?;
    Ok(())
}