use thiserror::Error;

/// Represents all possible errors that can occur within the `bdk-labels` crate.
#[derive(Debug, Error)]
pub enum Error {
    /// An error originating from standard filesystem or stream I/O operations.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// An error related to serializing or deserializing the BIP-329 JSONL format.
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    /// An error indicating that the provided label data violates the BIP-329 specification.
    #[error("Invalid BIP329 structure: {0}")]
    Validation(String),

    /// An opaque error type allowing consumers to bubble up errors from their custom database backends.
    #[error("Custom Database error: {0}")]
    Custom(Box<dyn std::error::Error + Send + Sync>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_io_error_conversion() {
        fn trigger_conversion() -> Result<(), Error> {
            let io_error = io::Error::new(io::ErrorKind::Other, "Forced IO Failure");
            Err(io_error)?
        }

        let result = trigger_conversion();

        assert!(matches!(result, Err(Error::Io(_))));
    }

    #[test]
    fn test_serde_json_error_conversion() {
        fn trigger_conversion() -> Result<(), Error> {
            let json_error =
                serde_json::from_str::<serde_json::Value>("{ corrupted json string}").unwrap_err();
            Err(json_error)?
        }

        let result = trigger_conversion();

        assert!(matches!(result, Err(Error::Json(_))));
    }
}
