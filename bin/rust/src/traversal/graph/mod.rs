pub mod protein_graph;
pub mod metadata;
pub mod traversal;

// Re-export the items you defined in submodules
pub use protein_graph::ProteinGraph;
pub use metadata::MetaData;
pub use traversal::TraversalData;
