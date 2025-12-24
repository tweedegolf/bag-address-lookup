#[derive(Debug)]
pub enum DatabaseError {
    NotFound,
    TooShort,
    InvalidMagic,
    InvalidLayout,
    DecompressionFailed,
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            DatabaseError::NotFound => "database file not found",
            DatabaseError::TooShort => "database file too short",
            DatabaseError::InvalidMagic => "database file has invalid magic",
            DatabaseError::InvalidLayout => "database file layout invalid",
            DatabaseError::DecompressionFailed => "database file decompression failed",
        };
        f.write_str(message)
    }
}

impl std::error::Error for DatabaseError {}
