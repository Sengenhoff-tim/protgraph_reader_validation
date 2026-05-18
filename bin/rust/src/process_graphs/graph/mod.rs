/// struct for protein graph as read from .bpcsr

mod protein_graph;
mod metadata;
mod traversal;

pub use protein_graph::ProteinGraph;
pub use metadata::MetaData;
pub use traversal::TraversalData;
