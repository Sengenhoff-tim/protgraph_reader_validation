use anyhow::{Result};

use crate::{traversal::{StringTable, Entry, EntryMeta}};

// Protein graph struct and utility functions.
// Compared to the original implementation, the max vars vector has been removed as per the requirements.

struct ForwardPass {
    seq: String,
    qualifiers: String,
    iso_idx: u8,
    mssclvg: u32,
    spos: Option<u16>,
    last_node_idx: Option<(usize, usize)>
}
pub struct MetaData {
    pub accessions: Vec<String>,
    pub position: Box<[u16]>,
    pub iso_position: Box<[u16]>,
    pub iso_index: Box<[u8]>,
    pub cleaved: Vec<bool>,
    pub sequences: StringTable,
    pub qualifiers: StringTable,
}

impl MetaData {

pub fn build_peptide(
    &self, 
    trace: &[(u32, u32)]
) -> Result<Option<Entry>> {
    let trace_len = trace.len();

    if trace_len < 2 {
        return Ok(None);
    }
    let mut fwd_res = self.forward_pass(trace, trace_len)?;

    // -------- final edge --------
    if trace[trace_len - 1].1 != u32::MAX {
        let q = self.qualifiers.get_str(trace[trace_len - 1].1 as usize);
        if !q.is_empty() {
            fwd_res.qualifiers.push_str(q);
            fwd_res.qualifiers.push(',');
        }
    }

    let mut epos: Option<u16> = None;
    // -------- compute epos (NO reverse scan) --------
    if let Some((node_idx, seq_len)) = fwd_res.last_node_idx {
        let iso_pos = self.iso_position[node_idx];
        
        if iso_pos != u16::MAX {
            epos = Some(iso_pos  + seq_len  as u16- 1)
        } else {
            let pos = self.position[node_idx];
            if pos != u16::MAX {
                epos = Some(pos + seq_len as u16 - 1)
            }
        };
    }

    let acc = self.accessions
        .get(fwd_res.iso_idx as usize)
        .map(|s| s.as_str())
        .unwrap_or("NOT FOUND");

    let qualifiers_str = fwd_res.qualifiers.strip_suffix(',').unwrap_or(&fwd_res.qualifiers);

    Ok(Some(Entry{
                pep: fwd_res.seq,
                meta: EntryMeta {
                    acc: acc.to_string(), 
                    qualifiers: qualifiers_str.to_string(),
                    spos: fwd_res.spos,
                    epos: epos,
                    mssclvg: fwd_res.mssclvg
                }
                
            
            }
        )
    )
}

fn forward_pass(
    &self,
    trace: &[(u32, u32)],
    trace_len: usize,
) -> Result<ForwardPass>{

    let mut seq_out = String::new();
    let mut qualifiers_out = String::new();
    let mut iso_idx: u8 = 0;
    let mut mssclvg: u32 = 0;
    
    let mut last_seq_node: Option<(usize, usize)> = None;

    let mut spos_retrieved = false;

    let mut spos = None;

    let mut count: u32 = 0;

    for &(node, edge) in &trace[1..trace_len - 1] {
        let node_idx = node as usize;

        let seq = self.sequences.get_str(node_idx).to_string();

        if seq.is_empty() {
            continue; 
        }
        let seq_len = seq.len();

        for ch in seq.chars() {
            seq_out.push(ch);
            count += 1;

            if count % 60 == 0 {
                seq_out.push('\n');
            }
        }

        
        // first non-empty → spos
        if !spos_retrieved {
            spos_retrieved = true;

            let iso_pos = self.iso_position[node_idx];
            

            if iso_pos != u16::MAX {
                spos = Some(iso_pos);
            } else {
                let pos = self.position[node_idx];
                if pos != u16::MAX {
                    spos = Some(pos);
                }
            }
        }

        // always update → gives last non-empty automatically
        last_seq_node = Some((node_idx, seq_len));

        iso_idx = iso_idx.max(self.iso_index[node_idx]);

        if edge != u32::MAX {
            if self.cleaved[edge as usize] {
                mssclvg += 1;
            }

            let q = self.qualifiers.get_str(edge as usize);
            if !q.is_empty() {
                qualifiers_out.push_str(q);
                qualifiers_out.push(',');
            }
        }
    }
    
    Ok(ForwardPass{
            seq: seq_out, 
            qualifiers: qualifiers_out, 
            iso_idx, 
            mssclvg, 
            spos,
            last_node_idx: last_seq_node, 
        })
}
}

   /*
pub fn write_peptide<W: Write>(
    &self,
    sequences: Arc<StringTable>,
    out: &mut W,
    trace: &[(u32, u32)],
) -> Result<()> {
    let len = trace.len();
    if len < 2 {
        return Ok(());
    }

    let mut sequence = Vec::new();
    let mut qualifiers = Vec::new();

    let mut iso_idx: u8 = 0;
    let mut mssclvg: u32 = 0;

    let mut spos = "?".to_string();
    let mut epos = "?".to_string();

    let mut spos_retrieved = false;

    let mut last_seq_node: Option<(usize, usize)> = None; // (node_idx, seq_len)

    // -------- single forward pass --------
    for &(node, edge) in &trace[1..len - 1] {
        let node_idx = node as usize;

        let seq = sequences.get_str(node_idx);
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
     */ 