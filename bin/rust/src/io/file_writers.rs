use std::io::{Write};
use crossbeam_channel::Receiver;

use crate::traversal::Entry;

pub fn writer_thread<W: Write>(
    rx_entry: Receiver<Entry>,
    writer: &mut W,
) -> anyhow::Result<()> {
    //, 
    for entry in rx_entry {
        let meta = entry.meta;
        let spos = meta.spos.map(|v| v.to_string()).unwrap_or_else(|| "?".to_string());
        let epos = meta.epos.map(|v| v.to_string()).unwrap_or_else(|| "?".to_string());
        writeln!(
            writer,
            ">pg|TODO|{}({}:{},mssclvg:{},{})\n{}",
            meta.acc,
            spos,
            epos,
            meta.mssclvg,
            meta.qualifiers,
            entry.pep
            )?;
    }

    writer.flush()?;
    Ok(())
}