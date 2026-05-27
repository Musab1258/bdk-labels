//! # bdk-labels
//!
//! A Rust library providing native BIP-329 (Wallet Labels Export Format) support
//! for the Bitcoin Dev Kit (BDK) ecosystem.
//!
//! This crate extends `bdk_wallet::Wallet` to allow developers to label transactions,
//! addresses, UTXOs, and public keys with human-readable labels. It provides a
//! deterministic, `BTreeMap`-backed memory structure (`LabelChangeset`) and a decoupled
//! persistence trait (`LabelPersister`) for seamless integration with any database backend.

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

/// A wrapper type for targeting a specific transaction output (UTXO).
pub struct OutputTarget(pub OutPoint);

/// A wrapper type for targeting a specific transaction input.
pub struct InputTarget(pub OutPoint);

/// Represents the various Bitcoin primitives that can be tagged with a BIP-329 label.
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

/// The core trait providing BIP-329 operations for a wallet.
pub trait Bip329 {
    /// Adds a new human-readable label to a specified target (e.g., Address, Txid).
    fn add_label(
        &mut self,
        target: impl Into<LabelTarget>,
        label_text: impl Into<String>,
    ) -> Result<Label, Error>;

    /// Imports labels from a BIP-329 compliant JSONL stream and merges them with the current state.
    fn import_labels<R: BufRead>(
        &mut self,
        reader: R,
        strategy: MergeStrategy,
    ) -> Result<(), Error>;

    /// Exports the current label state deterministically to a writable stream in BIP-329 JSONL format.
    fn export_labels<W: Write>(&self, writer: W) -> Result<(), Error>;
}
