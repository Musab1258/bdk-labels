use crate::changeset::LabelChangeset;
use crate::error::Error;

pub trait LabelPersister {
    fn read_labels(&self) -> Result<LabelChangeset, Error>;

    fn append_changeset(&mut self, changeset: &LabelChangeset) -> Result<(), Error>;
}
