pub mod error;

pub use error::Error;

use bip329::{Label, LabelRef};

pub trait Bip329 {
    fn add_label(&mut self, item_ref: LabelRef, label: String) -> Result<Label, Error>;

    fn import_label(&mut self, file: impl AsRef<std::path::Path>) -> Result<(), Error>;

    fn export_label(&self, file: impl AsRef<std::path::Path>) -> Result<(), Error>;
}
