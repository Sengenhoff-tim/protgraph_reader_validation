use serde::Deserialize;

//use crate::protgraph_types::Raw64;

// A helper struct for intervals of protein weights

#[derive(Clone, Debug, Deserialize)]
pub struct Interval {
    pub lower: i64,
    pub upper: i64,
}

impl Interval {
    pub fn overlaps(&self, other: &Interval) -> bool {
        self.lower <= other.upper && self.upper >= other.lower
    }
}

/* 
#[derive(Clone, Debug)]
pub struct Interval {
    pub lower: Raw64,
    pub upper: Raw64,
}

impl Interval {
    #[inline]
    pub fn overlaps(&self, other: &Interval) -> bool {
        self.lower.as_i64() <= other.upper.as_i64()
            && self.upper.as_i64() >= other.lower.as_i64()
    }
}
    */