# bdk-labels

`bdk-labels` is a Rust library that brings native **BIP-329 (Wallet Labels Export Format)** support to the [Bitcoin Dev Kit (BDK)](https://bitcoindevkit.org/) ecosystem. 

It provides a seamless extension to `bdk_wallet::Wallet`, allowing developers to easily tag transactions, addresses, UTXOs, public keys and extended public keys with human-readable labels, and persist them using their database backend of choice.

## Features

* **Native BDK Integration:** Extends `bdk_wallet::Wallet` seamlessly via the `LabelledWallet` wrapper.
* **Deterministic Exports:** Backed by a `BTreeMap`, label exports are alphabetically sorted by reference. This guarantees stable, deterministic JSONL outputs, preventing noisy diffs for users tracking their wallet metadata in version control (e.g., Git).
* **Pluggable Persistence:** Implement the `LabelPersister` trait to store your labels in SQLite, Redb, or any custom backend, completely decoupled from the main wallet database.
* **Smart Merging:** Built-in support for merging imported BIP-329 files with existing label state using configurable strategies (`Overwrite` or `KeepExisting`).

## Usage

### 1. Initializing a Labelled Wallet

Wrap your existing BDK `Wallet` and a `LabelChangeset` inside a `LabelledWallet` to access the BIP-329 API.

```rust
use bdk_labels::{LabelledWallet, LabelChangeset, Bip329};
use bdk_wallet::Wallet;
use bitcoin::Network;

// 1. Initialize your standard BDK wallet
let mut wallet = Wallet::create(external_desc, internal_desc)
    .network(Network::Testnet)
    .create_wallet_no_persist()
    .unwrap();

// 2. Load or initialize your label state
let mut labels = LabelChangeset::new();

// 3. Wrap it up
let mut labelled_wallet = LabelledWallet {
    wallet: &mut wallet,
    labels: &mut labels,
};

```

### 2. Adding Labels

You can add labels to Transactions (`Txid`), Addresses, Public Keys, Inputs, and Outputs. The library automatically handles the enum variant mapping for BIP-329 compliance.

```rust
use std::str::FromStr;
use bitcoin::{Txid, Address};

let txid = Txid::from_str("0000000000000000000000000000000000000000000000000000000000000000").unwrap();
let address = Address::from_str("mkHS9ne12qx9pS9VojpwU5xtRd4T7X7ZUt").unwrap().assume_checked();

// Label a transaction
labelled_wallet.add_label(txid, "Payment for Machinery").unwrap();

// Label an address
labelled_wallet.add_label(address, "Employee Address").unwrap();

```

### 3. Exporting and Importing (JSONL)

`bdk-labels` supports streaming labels to and from standard `BufRead` and `Write` traits, making it easy to interact with the filesystem or network streams.

**Exporting:**

```rust
use std::fs::File;

let mut file = File::create("my_wallet.labels.jsonl").unwrap();
labelled_wallet.export_labels(&mut file).unwrap();

```

*Note: Exported labels are deterministically sorted by their BIP-329 ref field, making the resulting file safe and clean for Git version control.*

**Importing:**
When importing labels from an external file, you must specify a `MergeStrategy` to handle collisions (when an imported label references the same item as an existing label).

* `MergeStrategy::Overwrite`: The imported label replaces the existing label.
* `MergeStrategy::KeepExisting`: The existing label is preserved, and the imported label is ignored.

```rust
use bdk_labels::MergeStrategy;
use std::io::BufReader;

let file = File::open("incoming.labels.jsonl").unwrap();
let reader = BufReader::new(file);

// Merge incoming labels, overwriting any existing collisions
labelled_wallet.import_labels(reader, MergeStrategy::Overwrite).unwrap();

```

## Implementing Persistence

To persist labels to disk, implement the `LabelPersister` trait for your database backend. This allows you to append `LabelChangeset` diffs efficiently.

```rust
use bdk_labels::{LabelPersister, LabelChangeset};

pub struct MySqlitePersister { /* ... */ }

impl LabelPersister for MySqlitePersister {
    type Error = MyCustomError;

    fn read_labels(&self) -> Result<LabelChangeset, Self::Error> {
        // Load your changeset from the database
    }

    fn append_changeset(&mut self, changeset: &LabelChangeset) -> Result<(), Self::Error> {
        // Write only the diffs to the database
    }
}

// Usage:
labelled_wallet.persist(&mut my_sqlite_persister).unwrap();

```

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Minimum Supported Rust Version (MSRV)

This crate supports Rust **1.70.0** or newer.

## Contributing
Found a bug, have an issue or a feature request? Feel free to open an issue on GitHub.
