#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Raw64(pub u64);

impl Raw64 {
    #[inline]
    pub fn as_i64(self) -> i64 {
        self.0 as i64
    }
}

impl Ord for Raw64 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_i64().cmp(&other.as_i64())
    }
}

impl PartialOrd for Raw64 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Raw64 {
    #[inline]
    pub fn wrapping_add(self, rhs: Raw64) -> Raw64 {
        Raw64(self.0.wrapping_add(rhs.0))
    }

    #[inline]
    pub fn wrapping_sub(self, rhs: Raw64) -> Raw64 {
        Raw64(self.0.wrapping_sub(rhs.0))
    }
}