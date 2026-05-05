use super::{MetaData, TraversalData};
use crate::traversal::StringTable;

pub struct ProteinGraph {
    pub traversal_data: TraversalData,
    pub meta_data: MetaData,
    pub sequences: StringTable,
}