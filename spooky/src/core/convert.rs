use anyhow::Result;

/// Trait for converting from an index
pub trait FromIndex: Sized {
    /// Convert from an index to Self
    fn from_index(idx: usize) -> Result<Self>;
}

/// Trait for converting to an index
pub trait ToIndex {
    /// Convert self to an index
    fn to_index(&self) -> Result<usize>;
}
