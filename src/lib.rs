pub mod changeset;
pub mod error;
pub mod extension;
pub mod io;
pub mod persist;

pub use changeset::{LabelChangeset, MergeStrategy};
pub use error::Error;
pub use extension::*;
pub use io::{export, import};
pub use persist::LabelPersister;

use bip329::Label;
use bitcoin::address::NetworkUnchecked;
use bitcoin::bip32::Xpub;
use bitcoin::{Address, OutPoint, PublicKey, Txid};
use std::io::{BufRead, Write};

pub struct OutputTarget(pub OutPoint);
pub struct InputTarget(pub OutPoint);

pub enum LabelTarget {
    Txid(Txid),
    Address(Address<NetworkUnchecked>),
    PublicKey(String),
    Input(OutPoint),
    Output(OutPoint),
    Xpub(String),
}

impl From<Txid> for LabelTarget {
    fn from(txid: Txid) -> Self {
        LabelTarget::Txid(txid)
    }
}

impl From<Address<NetworkUnchecked>> for LabelTarget {
    fn from(addr: Address<NetworkUnchecked>) -> Self {
        LabelTarget::Address(addr)
    }
}

impl From<PublicKey> for LabelTarget {
    fn from(pk: PublicKey) -> Self {
        LabelTarget::PublicKey(pk.to_string())
    }
}

impl From<InputTarget> for LabelTarget {
    fn from(input: InputTarget) -> Self {
        LabelTarget::Input(input.0)
    }
}

impl From<OutputTarget> for LabelTarget {
    fn from(output: OutputTarget) -> Self {
        LabelTarget::Output(output.0)
    }
}

impl From<Xpub> for LabelTarget {
    fn from(xp: Xpub) -> Self {
        LabelTarget::Xpub(xp.to_string())
    }
}

pub trait Bip329 {
    fn add_label(
        &mut self,
        target: impl Into<LabelTarget>,
        label_text: impl Into<String>,
    ) -> Result<Label, Error>;

    fn import_labels<R: BufRead>(
        &mut self,
        reader: R,
        strategy: MergeStrategy,
    ) -> Result<(), Error>;

    fn export_labels<W: Write>(&self, writer: W) -> Result<(), Error>;
}
