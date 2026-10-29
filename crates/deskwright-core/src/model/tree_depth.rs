use crate::error::{DeskwrightError, DeskwrightResult};

/// How many levels below each top-level element a snapshot reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeDepth(usize);

impl TreeDepth {
    pub const DEFAULT: Self = Self(10);
    pub const MAXIMUM: Self = Self(50);

    pub fn new(levels: usize) -> DeskwrightResult<Self> {
        if levels == 0 || levels > Self::MAXIMUM.0 {
            return Err(DeskwrightError::InvalidInput(format!(
                "tree depth must be between 1 and {}",
                Self::MAXIMUM.0
            )));
        }
        Ok(Self(levels))
    }

    pub fn levels(self) -> usize {
        self.0
    }
}

impl Default for TreeDepth {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_depths_from_one_to_the_maximum() {
        assert_eq!(TreeDepth::new(1).unwrap().levels(), 1);
        assert_eq!(TreeDepth::new(50).unwrap(), TreeDepth::MAXIMUM);
    }

    #[test]
    fn rejects_zero_and_depths_above_the_maximum() {
        assert!(TreeDepth::new(0).is_err());
        assert!(TreeDepth::new(51).is_err());
    }

    #[test]
    fn defaults_to_ten_levels() {
        assert_eq!(TreeDepth::default().levels(), 10);
    }
}
