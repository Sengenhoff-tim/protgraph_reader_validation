use crossbeam_channel::{Sender};
use std::io::{BufRead, ErrorKind};

use byteorder::{BigEndian, ReadBytesExt};
use anyhow::{Result, anyhow};

use crate::traversal::{Interval, Pdbs, ProteinGraph, StringTable, MetaData, TraversalData};

pub fn start_protein_graph_reader<R: BufRead>(
    rdr: R,
    tx_protgraph: Sender<Result<ProteinGraph>>,
) {
    let mut rdr = rdr;
    
    loop {
        let num_acc = match rdr.read_u32::<BigEndian>() {
            Ok(n) => n,
            Err(e) => {
                if e.kind() != ErrorKind::UnexpectedEof {
                    let _ = tx_protgraph.send(Err(anyhow!("{}", e)));
                }
                break;
            }
        };

        match read_single_graph(num_acc, &mut rdr) {
            Ok(pg) => {
                if tx_protgraph.send(Ok(pg)).is_err() {
                    break;
                }
            }
            Err(e) => {
                let _ = tx_protgraph.send(Err(e));
                break;
            }
        }
    }
}

fn read_single_graph<R: BufRead>(num_acc: u32, reader: &mut R) -> Result<ProteinGraph> {
        let n_acc = num_acc as usize;
        // Read counts (big-endian)
        let n_nodes = reader.read_u32::<BigEndian>()? as usize;
        let n_edges = reader.read_u32::<BigEndian>()? as usize;
        let n_pdbs  = reader.read_u32::<BigEndian>()? as usize;

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
            traversal_data: TraversalData{
                nodes: nodes.into_boxed_slice(),
                edges: edges.into_boxed_slice(),
                mono_weight: mono_weight.into_boxed_slice(),
                variant_count: variant_count.into_boxed_slice(),
                pdbs,
            },
            meta_data: MetaData{
                accessions,
                position: position.into_boxed_slice(),
                iso_index: iso_index.into_boxed_slice(),
                iso_position: iso_position.into_boxed_slice(),
                cleaved,
                sequences,
                qualifiers,
            },
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
    read_be_vec(reader, count, 1, |b| b[0])
}

fn read_u16_vec<R: BufRead>(reader: &mut R, count: usize) -> Result<Vec<u16>> {
    read_be_vec(reader, count, 2, |b| {
        u16::from_be_bytes([b[0], b[1]])
    })
}

// check for u32::MAX which is used as sentinel in TraversalData
fn read_u32_vec<R: BufRead>(reader: &mut R, count: usize) -> Result<Vec<u32>> {
    let mut out = Vec::with_capacity(count);
    let mut buf = [0u8; 4];

    for i in 0..count {
        reader.read_exact(&mut buf)?;
        let value = u32::from_be_bytes(buf);

        if value == u32::MAX {
           return Err(anyhow!(
                "u32::MAX at index {} (reserved sentinel; possible overflow/truncation)",
                i
            ));
        }

        out.push(value);
    }

    Ok(out)
}

fn read_be_vec<R: BufRead, T>(
    reader: &mut R,
    count: usize,
    byte_len: usize,
    parse: fn(&[u8]) -> T,
) -> Result<Vec<T>> {
    let total = count
        .checked_mul(byte_len)
        .ok_or_else(|| anyhow!("overflow in allocation size"))?;
    let mut buf = vec![0u8; total];
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
            let raw_lower = reader.read_u64::<BigEndian>()?;
            let raw_upper = reader.read_u64::<BigEndian>()?;

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

            // skip invalid
            if lower == i64::MAX {
                continue;
            }

            // optional validation (can be removed for max speed)
            if upper < lower {
                // either skip or fix depending on your semantics
                continue;
            }

            node_vec.push(Interval { lower, upper });
        }

        // IMPORTANT: sort for early-exit scan optimization
        node_vec.sort_unstable_by_key(|iv| iv.lower);

        node_lists.push(node_vec);
    }

    Ok(Pdbs::from_node_lists(node_lists)?)
}