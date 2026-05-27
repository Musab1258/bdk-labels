use crate::changeset::LabelChangeset;

/// A trait outlining the required operations for persisting a `LabelChangeset`
/// to a long-term storage backend (e.g., SQLite, Redb, Postgres).
pub trait LabelPersister {
    /// The error type returned by the storage backend implementation.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Retrieves the entire label state from the database and constructs a `LabelChangeset`.
    fn read_labels(&self) -> Result<LabelChangeset, Self::Error>;

    /// Appends or updates the database with the differences contained in the provided `LabelChangeset`.
    fn append_changeset(&mut self, changeset: &LabelChangeset) -> Result<(), Self::Error>;
}
