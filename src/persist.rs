use crate::changeset::LabelChangeset;

pub trait LabelPersister {
    type Error: std::error::Error + Send + Sync + 'static;

    fn read_labels(&self) -> Result<LabelChangeset, Self::Error>;

    fn append_changeset(&mut self, changeset: &LabelChangeset) -> Result<(), Self::Error>;
}
