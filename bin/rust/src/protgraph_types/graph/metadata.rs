use std::io::{Write};
use anyhow::{Result};

use crate::{protgraph_types::{StringTable}};

// Protein graph struct and utility functions.
// Compared to the original implementation, the max vars vector has been removed as per the requirements.


pub struct MetaData {
    pub accessions: Vec<String>,
    pub position: Box<[u16]>,
    pub iso_position: Box<[u16]>,
    pub iso_index: Box<[u8]>,
    pub cleaved: Vec<bool>,
    pub qualifiers: StringTable,
    pub sequences: StringTable,
}

impl MetaData {
    
pub fn write_fragment<W: Write>(
    &self,
    out: &mut W,
    trace: &[(u32, u32)],
) -> Result<()> {
    let len = trace.len();
    if len < 2 {
        return Ok(());
    }

    let mut sequence = String::with_capacity(256);
    let mut qualifiers = String::with_capacity(128);

    let mut iso_idx: u8 = 0;
    let mut mssclvg: u32 = 0;

    let mut spos = "?".to_string();
    let mut epos = "?".to_string();

    let mut spos_retrieved = false;

    // NEW: track last sequence-bearing node
    let mut last_seq_node: Option<(usize, usize)> = None; // (node_idx, seq_len)

    // -------- single forward pass --------
    for &(node, edge) in &trace[1..len - 1] {
        let node_idx = node as usize;

        let seq = self.sequences.get_str(node_idx);
        if !seq.is_empty() {
            let seq_len = seq.len();

            sequence.push_str(seq);

            // first non-empty → spos
            if !spos_retrieved {
                spos_retrieved = true;

                let iso_pos = self.iso_position[node_idx];
                let pos = self.position[node_idx];

                spos = if iso_pos != u16::MAX {
                    iso_pos.to_string()
                } else if pos != u16::MAX {
                    pos.to_string()
                } else {
                    "?".to_string()
                };
            }

            // always update → gives last non-empty automatically
            last_seq_node = Some((node_idx, seq_len));
        }

        iso_idx = iso_idx.max(self.iso_index[node_idx]);

        if edge != u32::MAX {
            if self.cleaved[edge as usize] {
                mssclvg += 1;
            }

            let q = self.qualifiers.get_str(edge as usize);
            if !q.is_empty() {
                qualifiers.push_str(q);
                qualifiers.push(',');
            }
        }
    }

    // -------- final edge --------
    if trace[len - 1].1 != u32::MAX {
        let q = self.qualifiers.get_str(trace[len - 1].1 as usize);
        if !q.is_empty() {
            qualifiers.push_str(q);
            qualifiers.push(',');
        }
    }

    // -------- compute epos (NO reverse scan) --------
    if let Some((node_idx, seq_len)) = last_seq_node {
        let iso_pos = self.iso_position[node_idx];
        let pos = self.position[node_idx];

        epos = if iso_pos != u16::MAX {
            (iso_pos as usize + seq_len - 1).to_string()
        } else if pos != u16::MAX {
            (pos as usize + seq_len - 1).to_string()
        } else {
            "?".to_string()
        };
    }

    let acc = self.accessions
        .get(iso_idx as usize)
        .map(|s| s.as_str())
        .unwrap_or("TODO");

    let qualifiers_str = qualifiers.strip_suffix(',').unwrap_or(&qualifiers);

    writeln!(
        out,
        ">pg|TODO|{}({}:{},mssclvg:{},{})",
        acc, spos, epos, mssclvg, qualifiers_str
    )?;

    for chunk in sequence.as_bytes().chunks(60) {
        out.write_all(chunk)?;
        out.write_all(b"\n")?;
    }

    Ok(())
}
}