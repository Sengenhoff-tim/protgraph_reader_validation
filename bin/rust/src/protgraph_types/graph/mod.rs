pub mod protein_graph;
pub mod graph_metadata;
pub mod graph_traversal;

// Re-export the items you defined in submodules
pub use protein_graph::ProteinGraph;
pub use graph_metadata::MetaData;
pub use graph_traversal::TraversalData;
