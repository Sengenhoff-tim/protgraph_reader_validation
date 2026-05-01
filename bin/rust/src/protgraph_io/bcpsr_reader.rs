use std::io::{BufRead, ErrorKind};
use byteorder::{BigEndian, ReadBytesExt};
use anyhow::{Result, anyhow};

use crate::protgraph_types::{ProteinGraph, StringTable, Pdbs, Interval};

// Reader for the bpcsr binary files produced by ProtGraph. Implementation closely resembles the original for correctness.
// The max vars vector is never build as per the requirements.

pub struct ProteinGraphReader<R: BufRead> {
    rdr: R,
    finished: bool,
}

impl<R: BufRead> ProteinGraphReader<R> {
    pub fn new(rdr: R) -> Self {
        Self { rdr, finished: false }
    }
}

impl<R: BufRead> Iterator for ProteinGraphReader<R> {
    type Item = Result<ProteinGraph>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        let num_acc = match self.rdr.read_u32::<BigEndian>() {
            Ok(n) => n,
            Err(e) => {
                self.finished = true;
                return match e.kind() {
                    ErrorKind::UnexpectedEof => None,
                    _ => Some(Err(anyhow!("{}", e))),
                };
            }
        };

        match read_single_graph(num_acc, &mut self.rdr) {
            Ok(pg) => Some(Ok(pg)),
            Err(e) => {
                self.finished = true;
                
                if e.downcast_ref::<std::io::Error>()
                    .is_some_and(|ioe| ioe.kind() == ErrorKind::UnexpectedEof)
                {
                    return None;
                }
                
                Some(Err(e))
            }
        }
    }
}

fn read_single_graph<R: BufRead>(num_acc: u32, reader: &mut R) -> Result<ProteinGraph> {
        let n_acc = num_acc as usize;
        // Read counts (big-endian)
        let n_nodes = reader.read_u32::<BigEndian>()? as usize;
        let n_edges = reader.read_u32::<BigEndian>()? as usize; //not used, reader needs to progress
        let n_pdbs  = reader.read_u32::<BigEndian>()? as usize; //not used, reader needs to progress

        // Accessions (AC): num_acc NUL-terminated strings
        let mut accessions: Vec<String> = Vec::with_capacity(n_acc);
        for _ in 0..n_acc {
            accessions.push(read_cstring(reader)?);
        }

        // Nodes (NO): n_nodes u32 BE
        let nodes = read_u32_vec(reader, n_nodes)?;

        // Edges (ED): n_edges u32 BE
        let edges = read_u32_vec(reader, n_edges)?;

        let sequences = build_from_reader(reader, n_nodes)?;

        // Position (PO): n_nodes u16 BE
        let position = read_u16_vec(reader, n_nodes)?;

        // Iso index (IS): n_nodes u8
        let iso_index = read_u8_vec(reader, n_nodes)?;

        // Iso position (IP): n_nodes u16 BE
        let iso_position = read_u16_vec(reader, n_nodes)?;

        // Mono weight (MW): n_nodes i64 BE
        let mut mono_weight = Vec::with_capacity(n_nodes);
        for _ in 0..n_nodes {
            mono_weight.push(reader.read_i64::<BigEndian>()?);
        }

        // Cleaved (CL): n_edges bytes -> bool
        let mut cleaved = vec![false; n_edges];
        for i in 0..n_edges {
            cleaved[i] = reader.read_u8()? != 0;
        }

        let qualifiers = build_from_reader(reader, n_edges)?;

        // Variant count (VC): n_edges u8
        let variant_count = read_u8_vec(reader, n_edges)?;

        // PDBs: n_nodes * n_pdbs
        let pdbs = read_pdbs(reader, n_nodes, n_pdbs)?;


        Ok(ProteinGraph {
            accessions,
            nodes: nodes.into_boxed_slice(),
            edges: edges.into_boxed_slice(),
            sequences,
            position: position.into_boxed_slice(),
            iso_index: iso_index.into_boxed_slice(),
            iso_position: iso_position.into_boxed_slice(),
            mono_weight: mono_weight.into_boxed_slice(),
            cleaved,
            qualifiers,
            variant_count: variant_count.into_boxed_slice(),
            pdbs,
        })
    }

fn build_from_reader<R: BufRead>(reader: &mut R, count: usize) -> Result<StringTable> {
        let mut items = Vec::with_capacity(count);

        for _ in 0..count {
            items.push(read_cstring(reader)?);
        }

        Ok(StringTable::build_from_strings(items))
    }


fn read_u8_vec<R: BufRead>(reader: &mut R, count: usize) -> Result<Vec<u8>> {
    read_be_vec(reader, count, 1, |b| u8::from_be_bytes(b.try_into().unwrap()))
}

fn read_u16_vec<R: BufRead>(reader: &mut R, count: usize) -> Result<Vec<u16>> {
    read_be_vec(reader, count, 2, |b| u16::from_be_bytes(b.try_into().unwrap()))
}

fn read_u32_vec<R: BufRead>(reader: &mut R, count: usize) -> Result<Vec<u32>> {
    read_be_vec(reader, count, 4, |b| u32::from_be_bytes(b.try_into().unwrap()))
}

fn read_be_vec<R: BufRead, T>(
    reader: &mut R,
    count: usize,
    byte_len: usize,
    parse: fn(&[u8]) -> T,
) -> Result<Vec<T>> {
    let mut buf = vec![0u8; count * byte_len];
    reader.read_exact(&mut buf)?;

    let mut out = Vec::with_capacity(count);

    for chunk in buf.chunks_exact(byte_len) {
        out.push(parse(chunk));
    }

    Ok(out)
}

pub fn read_cstring<R: BufRead>(reader: &mut R) -> Result<String> {
    let mut bytes = Vec::new();
    reader.read_until(0u8, &mut bytes)?; // read up to and including NUL
    if matches!(bytes.last(), Some(0)) { bytes.pop(); } // drop terminating NUL
    Ok(bytes.into_iter().map(|b| b as char).collect())
}

pub fn read_pdbs<R: BufRead>(
    reader: &mut R,
    n_nodes: usize,
    n_pdbs: usize,
) -> Result<Pdbs> {
    let mut node_lists: Vec<Vec<Interval>> = Vec::with_capacity(n_nodes);

    for _node in 0..n_nodes {
        let mut node_vec = Vec::with_capacity(n_pdbs);

        for _slot in 0..n_pdbs {
            // Read as u64 to match C++ behavior
            let raw_lower = reader.read_u64::<BigEndian>()?;
            let raw_upper = reader.read_u64::<BigEndian>()?;

            // Map sentinel (uint64_t(-1)) -> i64::MAX
            let lower = if raw_lower == u64::MAX {
                i64::MAX
            } else {
                raw_lower as i64
            };

            let upper = if raw_upper == u64::MAX {
                i64::MAX
            } else {
                raw_upper as i64
            };

            // Only keep valid intervals
            if lower != i64::MAX {
                node_vec.push(Interval { lower, upper });
            }
        }

        node_lists.push(node_vec);
    }

    Ok(Pdbs::from_node_lists(node_lists)?)
}