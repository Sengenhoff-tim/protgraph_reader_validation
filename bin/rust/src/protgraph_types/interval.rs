use serde::Deserialize;

// A helper struct for intervals of protein weights

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Interval {
    pub lower: i64,
    pub upper: i64,
}

impl Interval {
    #[inline]
    pub fn overlaps(&self, other: &Interval) -> bool {
        self.lower <= other.upper && other.lower <= self.upper
    }

    #[inline]
    pub fn merge_with(&mut self, other: &Interval) {
        self.lower = self.lower.min(other.lower);
        self.upper = self.upper.max(other.upper);
    }
}

impl Interval {
    pub fn split_into_chunks(&self, chunk_size: i64) -> Vec<Interval> {
        let mut result = Vec::new();

        let mut start = self.lower;

        while start <= self.upper {
            let end = (start + chunk_size - 1).min(self.upper);

            result.push(Interval {
                lower: start,
                upper: end,
            });

            start = end + 1;
        }

        result
    }
}

pub trait IntervalVecExt {
    fn to_chunks(self, chunk_size: i64) -> Vec<Interval>;
}

impl IntervalVecExt for Vec<Interval> {
    fn to_chunks(mut self, chunk_size: i64) -> Vec<Interval> {
        if self.is_empty() {
            return vec![];
        }

        // O(n log n)
        self.sort_by_key(|i| i.lower);

        let mut result = Vec::new();
        let mut current = self[0];

        for interval in self.into_iter().skip(1) {
            if current.overlaps(&interval) {
                current.merge_with(&interval);
            } else {
                result.extend(current.split_into_chunks(chunk_size));
                current = interval;
            }
        }

        result.extend(current.split_into_chunks(chunk_size));

        result
    }
}