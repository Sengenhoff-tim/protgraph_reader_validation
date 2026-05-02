use serde::Deserialize;

// A helper struct for intervals of protein weights

#[derive(Clone, Debug, Deserialize)]
pub struct Interval {
    pub lower: i64,
    pub upper: i64,
}