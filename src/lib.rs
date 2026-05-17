pub mod changeset;
pub mod error;
pub mod extension;
pub mod io;

pub use error::Error;
pub use io::{export, import};

use crate::changeset::MergeStrategy;
use bip329::{Label, LabelRef};
use std::io::{BufRead, Write};

pub trait Bip329 {
    fn add_label(
        &mut self,
        target: impl Into<LabelRef>,
        label_text: impl Into<String>,
    ) -> Result<Label, Error>;

    fn import_labels<R: BufRead>(
        &mut self,
        reader: R,
        strategy: MergeStrategy,
    ) -> Result<(), Error>;

    fn export_labels<W: Write>(&self, writer: W) -> Result<(), Error>;
}
