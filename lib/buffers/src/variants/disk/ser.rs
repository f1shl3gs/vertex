/// Error that occurred during serialization.
#[derive(Debug)]
pub enum SerializeError<T> {
    /// The type failed to be serialized correctly.
    FailedToSerialize(String),

    /// The backing store was not big enough to fit the serialized version of the value.
    ///
    /// The original value that was given is returned, along with the minimum size that the backing
    /// store must be sized to hold the serialized value.  Providing a backing store that is larger
    /// than the given value is acceptable, but not necessary.
    BackingStoreTooSmall(T, usize),
}

/// Error that occurred during deserialization.
#[derive(Debug)]
pub enum DeserializeError {
    /// The data in the backing store does not represent the archive type as whole.
    ///
    /// This error is primarily indicative of not having enough data present, which is often a
    /// signal that the type represented by the bytes in the backing store and the incoming archive
    /// type are either entirely different, or that the structure of the type has changed: addition
    /// or removal of fields, reordering of fields, etc.
    ///
    /// The backing store that was given is returned, along with an error string that briefly
    /// describes the error in a more verbose fashion, suitable for debugging.
    InvalidStructure(String),

    /// Some of the data in the backing store cannot represent a particular field in the archive type.
    ///
    /// This would typically occur if the data read for a particular field could not specifically
    /// represent that type.  For example, a boolean is encoded as a single byte with a 0 or 1 as
    /// the value, so a value between 2 and 255 is inherently invalid for representing a boolean.
    ///
    /// This can be a subtle difference from `InvalidStructure`, but is primarily indicative of
    /// in-place data corruption, or data being overwritten by an outside process.
    ///
    /// The backing store that was given is returned, along with an error string that briefly
    /// describes the error in a more verbose fashion, suitable for debugging.
    InvalidData(String),

    /// The input buffer is too short to read metadata and payload.
    TooShort,
}

impl DeserializeError {
    /// Consumes this error and returns the stringified error reason.
    pub fn into_inner(self) -> String {
        match self {
            DeserializeError::InvalidData(s) => format!("invalid data: {}", s),
            DeserializeError::InvalidStructure(s) => format!("invalid structure: {}", s),
            DeserializeError::TooShort => "data too short for metadata and payload".to_string(),
        }
    }
}
