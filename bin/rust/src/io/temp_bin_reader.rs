use std::io::{Write, BufWriter};
use crossbeam_channel::{Receiver,Sender};
use serde::{Deserialize, Serialize};
use std::io::{Read};
use anyhow::{Result};

use crate::traversal::{MetaData, SerializedTraversalState, SerializableState, TraversalState, StateId};
pub enum Incoming {
    Meta(MetaData),
    Traversal(TraversalState),
}

#[derive(Serialize, Deserialize)]
enum Record {
    MetaData(MetaData),

    StateRow {
        node: u32,
        edge: u32,
        parent: StateId,
    },

    StatesAtNode(Vec<StateId>),
}

const MAX_BUF_SIZE: usize = 64 * 1024;

const ROW_BATCH_SIZE: usize = 4096;

pub struct StateRowWriter<W: Write> {
    writer: BufWriter<W>,
    buf: Vec<SerializableState>,
}

impl<W: Write> StateRowWriter<W> {
    pub fn new(inner: W) -> Self {
        Self {
            writer: BufWriter::new(inner),
            buf: Vec::with_capacity(ROW_BATCH_SIZE),
        }
    }

    #[inline]
    fn flush_rows(&mut self) -> bincode::Result<()> {
        if self.buf.is_empty() {
            return Ok(());
        }

        // serialize whole batch at once
        bincode::serialize_into(&mut self.writer, &self.buf)
            .map_err(|e| e)?;

        self.buf.clear();
        Ok(())
    }

    #[inline]
    pub fn push_row(&mut self, row: SerializableState) -> bincode::Result<()> {
        self.buf.push(row);

        if self.buf.len() >= ROW_BATCH_SIZE {
            self.flush_rows()?;
        }

        Ok(())
    }

    pub fn finish(mut self) -> bincode::Result<()> {
        self.flush_rows()?;
        self.writer.flush()?;
        Ok(())
    }
}

pub fn tmp_bin_writer<W: Write>(
    rx: Receiver<Incoming>,
    writer: W,
) -> Result<()> {
    let mut out = BufWriter::new(writer);
    let mut row_buf = StateRowWriter::new(Vec::new()); // or split writer if needed

    for msg in rx {
        match msg {
            Incoming::Meta(meta) => {
                // flush rows before switching context
                row_buf.finish()?;

                bincode::serialize_into(&mut out, &Record::MetaData(meta))?;
            }

            Incoming::Traversal(traversal) => {
                // buffer all rows
                for s in traversal.arena {
                    row_buf.push_row(SerializableState {
                        node: s.node,
                        edge: s.edge,
                        parent: s.parent,
                    })?;
                }

                // flush rows once before writing metadata-like tail
                row_buf.finish()?;

                let final_state =
                    traversal.states_at_node[traversal.final_state_idx].clone();

                bincode::serialize_into(
                    &mut out,
                    &Record::StatesAtNode(final_state),
                )?;
            }
        }
    }

    row_buf.finish()?;
    out.flush()?;
    Ok(())
}

pub fn tmp_bin_reader<R: Read>(
    mut reader: R,
    tx: Sender<SerializedTraversalState>,
) -> bincode::Result<()> {

    loop {
        // 1. read Meta (batch start)
        let meta: MetaData = match bincode::deserialize_from(&mut reader) {
            Ok(m) => m,
            Err(_) => break,
        };

        let _meta = meta; // optional use

        // 2. read traversal states until next Meta or EOF
        loop {
            // peek next record
            let record: Record = match bincode::deserialize_from(&mut reader) {
                Ok(r) => r,
                Err(_) => return Ok(()),
            };

            match record {
                Record::MetaData(_) => {
                    // next batch begins → restart outer loop
                    break;
                }

                Record::StateRow { node, edge, parent } => {
                    let mut arena = vec![
                        SerializableState { node, edge, parent }
                    ];

                    let mut states_at_node = None;

                    // consume rest of traversal
                    loop {
                        let r: Record = bincode::deserialize_from(&mut reader)?;

                        match r {
                            Record::StateRow { node, edge, parent } => {
                                arena.push(SerializableState { node, edge, parent });
                            }

                            Record::StatesAtNode(s) => {
                                states_at_node = Some(s);
                                break;
                            }

                            Record::MetaData(_) => {
                                return Err(bincode::ErrorKind::Custom(
                                    "Meta inside traversal".into(),
                                ).into());
                            }
                        }
                    }

                    let traversal = SerializedTraversalState {
                        arena,
                        final_states: states_at_node.unwrap(),
                    };

                    // BACKPRESSURE POINT
                    tx.send(traversal).ok();
                }

                Record::StatesAtNode(_) => {
                    return Err(bincode::ErrorKind::Custom(
                        "orphan StatesAtNode".into(),
                    ).into());
                }
            }
        }
    }

    Ok(())
}