use std::{i64};
use std::io::{Write};
use anyhow::{Result};
use std::fs::File;

use crate::{protgraph_types::{Pdbs, StringTable, Interval}};//, Raw64}};

// Protein graph struct and utility functions.
// Compared to the original implementation, the max vars vector has been removed as per the requirements.

pub struct ProteinGraph {
    pub num_n: u32,
    //pub num_e: u32,
    //pub num_pdbs: u32,
    pub accessions: Vec<String>,
    pub nodes: Box<[u32]>,
    pub edges: Box<[u32]>,
    pub sequences: StringTable,
    pub position: Box<[u16]>,
    pub iso_index: Box<[u8]>,
    pub iso_position: Box<[u16]>,
    pub mono_weight: Box<[i64]>,
    pub cleaved: Vec<bool>,
    pub qualifiers: StringTable,
    pub variant_count: Box<[u8]>,
    pub pdbs: Pdbs,
}

impl ProteinGraph {
    
    //DEBUG
    pub fn write_to_file_compact(&self, path: &str) -> anyhow::Result<()> {
    let filename = format!("{}{}", self.accessions.join("_"), path);
    let mut file = File::create(filename)?;

    let n_nodes = self.pdbs.offsets.len() - 1;
    writeln!(file, "# {} nodes", n_nodes)?;
    
    for node in 0..n_nodes {
        if let Some(intervals) = self.pdbs.get_node_intervals(node) {
            write!(file, "node_{}: ", node)?;
            for (idx, interval) in intervals.iter().enumerate() {
                if idx > 0 {
                    write!(file, " | ")?;
                }
                write!(file, "[{},{}]", interval.lower, interval.upper)?;
            }
            writeln!(file)?;
        }
    }
    
    Ok(())
}
    pub fn get_edge_index(&self, source_node: u32, target_node: u32) -> i32 {
        let start_idx = if source_node != 0 {
            self.nodes[(source_node - 1) as usize]
        } else {
            0
        } as usize;

        let end_idx = self.nodes[source_node as usize] as usize;

        if start_idx >= end_idx || end_idx > self.edges.len() {  // Changed >= and >
            return 0;
        }

        for idx in start_idx..end_idx {  // Changed from ..= to ..
            if self.edges[idx] == target_node {
                return idx as i32;
            }
        }

        0
    }

    pub fn has_overlapping_interval(
        &self,
        node: usize,
        interval: &Interval,
    ) -> anyhow::Result<bool> {
        let slice = self.pdbs
            .get_node_intervals(node)
            .ok_or_else(|| anyhow::anyhow!("invalid node index: {}", node))?;

        Ok(slice.iter().any(|iv| iv.overlaps(interval)))
    }

pub fn write_fragment_to_fasta<W: Write>(
    &self,
    out: &mut W,
    path: &[u32],
) -> Result<()> {
    if path.len() < 2 {
        return Ok(());
    }

    let mut sequence = String::new();
    let mut qualifiers = String::new();
    let mut iso_idx: u8 = 0;
    let mut mssclvg: u32 = 0;
    let mut spos = "?".to_string();
    let mut epos = "?".to_string();
    let mut spos_retrieved = false;

    // Process internal nodes and their edges
    for idx in 1..path.len() - 1 {
        let node = path[idx] as usize;
        let seq = self.sequences.get_str(node)?;

        // Accumulate sequence
        if !seq.is_empty() {
            sequence.push_str(seq);

            // Get start position on first non-empty sequence
            if !spos_retrieved {
                spos_retrieved = true;
                let iso_pos = self.iso_position[node];
                let pos = self.position[node];
                spos = if iso_pos != u16::MAX {
                    iso_pos.to_string()
                } else if pos != u16::MAX {
                    pos.to_string()
                } else {
                    "?".to_string()
                };
            }
        }

        // Track max iso_idx
        iso_idx = iso_idx.max(self.iso_index[node]);

        // Process edge from previous node to current node
        let prev = path[idx - 1];
        let cur = path[idx];
        let edge = self.get_edge_index(prev, cur) as usize;

        if self.cleaved[edge] {
            mssclvg += 1;
        }

        // Collect qualifier for this edge
        if let Ok(q) = self.qualifiers.get_str(edge) {
            if !q.is_empty() {
                qualifiers.push_str(q);
                qualifiers.push(',');
            }
        }
    }

    // Edge case: process the final edge (from path[len-2] to path[len-1])
    let final_edge = self.get_edge_index(path[path.len() - 2], path[path.len() - 1]) as usize;
    if let Ok(q) = self.qualifiers.get_str(final_edge) {
        if !q.is_empty() {
            qualifiers.push_str(q);
            qualifiers.push(',');
        }
    }

    // Find end position (reverse scan from path.len()-2 down to 1, exclusive of 0)
    let mut idx = path.len() - 2;
    loop {
        let node = path[idx] as usize;
        if let Ok(seq) = self.sequences.get_str(node) {
            if !seq.is_empty() {
                let len = seq.len() as u16;
                let iso_pos = self.iso_position[node];
                let pos = self.position[node];
                epos = if iso_pos != u16::MAX {
                    (iso_pos as usize + len as usize - 1).to_string()
                } else if pos != u16::MAX {
                    (pos as usize + len as usize - 1).to_string()
                } else {
                    "?".to_string()
                };
                break;
            }
        }
        if idx == 1 {
            break;
        }
        idx -= 1;
    }

    let acc = self.accessions
        .get(iso_idx as usize)
        .map(|s| s.as_str())
        .unwrap_or("TODO");

    // Remove trailing comma from qualifiers if present
    let qualifiers_str = if qualifiers.ends_with(',') {
        &qualifiers[..qualifiers.len() - 1]
    } else {
        &qualifiers
    };

    writeln!(
        out,
        ">pg|TODO|{}({}:{},mssclvg:{},{})",
        acc, spos, epos, mssclvg, qualifiers_str
    )?;

    // Write sequence in 60-character chunks
    for chunk in sequence.as_bytes().chunks(60) {
        out.write_all(chunk)?;
        out.write_all(b"\n")?;
    }

    Ok(())
}




}

