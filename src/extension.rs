use crate::{Bip329, LabelTarget};
use bdk_wallet::Wallet;

use crate::Error;
use crate::changeset::{LabelChangeset, MergeStrategy};
use crate::persist::LabelPersister;
use crate::{export, import};
use bip329::{Label, LabelRef};
use std::io::{BufRead, Write};

/// A wrapper around a BDK `Wallet` and a `LabelChangeset` that provides BIP-329 functionality.
///
/// This struct enables direct manipulation of wallet labels while holding mutable references
/// to both the underlying wallet state and the current label changeset.
pub struct LabelledWallet<'a> {
    /// A mutable reference to the underlying BDK wallet.
    pub wallet: &'a mut Wallet,
    /// A mutable reference to the current in-memory label state.
    pub labels: &'a mut LabelChangeset,
}

impl Bip329 for LabelledWallet<'_> {
    fn add_label(
        &mut self,
        target: impl Into<LabelTarget>,
        label_text: impl Into<String>,
    ) -> Result<Label, Error> {
        let new_label = match target.into() {
            LabelTarget::Txid(txid) => {
                let known_txid = self.wallet.list_output().any(|o| o.outpoint.txid == txid);
                if !known_txid {
                    return Err(Error::Validation(format!(
                        "txid {txid} not found in wallet"
                    )));
                }
                Label::Transaction(bip329::TransactionRecord {
                    ref_: txid,
                    label: Some(label_text.into()),
                    origin: None,
                })
            }
            LabelTarget::Address(addr) => {
                let checked_address = addr.assume_checked_ref();
                if !self.wallet.is_mine(checked_address.script_pubkey()) {
                    return Err(Error::Validation(format!(
                        "address {:?} does not belong to this wallet",
                        addr
                    )));
                }
                Label::Address(bip329::AddressRecord {
                    ref_: addr,
                    label: Some(label_text.into()),
                })
            }
            LabelTarget::PublicKey(pk) => Label::PublicKey(bip329::PublicKeyRecord {
                ref_: pk,
                label: Some(label_text.into()),
            }),
            LabelTarget::Input(outpoint) => {
                let known_input = self.wallet.list_output().any(|o| o.outpoint == outpoint);
                if !known_input {
                    return Err(Error::Validation(format!(
                        "outpoint {outpoint} not found in wallet"
                    )));
                }
                Label::Input(bip329::InputRecord {
                    ref_: outpoint,
                    label: Some(label_text.into()),
                })
            }
            LabelTarget::Output(outpoint) => {
                let known_input = self.wallet.list_output().any(|o| o.outpoint == outpoint);
                if !known_input {
                    return Err(Error::Validation(format!(
                        "outpoint {outpoint} not found in wallet"
                    )));
                }

                let spendable = self
                    .labels
                    .get(&LabelRef::Output(outpoint))
                    .and_then(|l| match l {
                        Label::Output(rec) => Some(rec.spendable),
                        _ => None,
                    })
                    .unwrap_or(true);

                Label::Output(bip329::OutputRecord {
                    ref_: outpoint,
                    label: Some(label_text.into()),
                    spendable,
                })
            }
            LabelTarget::Xpub(xpub) => Label::ExtendedPublicKey(bip329::ExtendedPublicKeyRecord {
                ref_: xpub,
                label: Some(label_text.into()),
            }),
        };

        self.labels.insert(new_label.clone());

        Ok(new_label)
    }

    fn import_labels<R: BufRead>(
        &mut self,
        reader: R,
        strategy: MergeStrategy,
    ) -> Result<(), Error> {
        let imported_labels = import(reader)?;
        self.labels.merge(imported_labels, strategy);
        Ok(())
    }

    fn export_labels<W: Write>(&self, writer: W) -> Result<(), Error> {
        export(self.labels, writer)
    }
}

impl LabelledWallet<'_> {
    /// Flushes only the labels changed since the last successful persist to
    /// the provided database persister.
    pub fn persist<P: LabelPersister>(&mut self, persister: &mut P) -> Result<(), Error> {
        if !self.labels.has_staged_changes() {
            return Ok(());
        }
        let diff = self.labels.diff();
        persister
            .append_changeset(&diff)
            .map_err(|e| Error::Custom(Box::new(e)))?;
        self.labels.clear_staged();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::changeset::LabelChangeset;
    use crate::{InputTarget, OutputTarget};
    use bdk_wallet::test_utils::{get_funded_wallet, get_test_wpkh_and_change_desc};
    use bdk_wallet::{KeychainKind, Wallet};
    use bip329::{
        AddressRecord, ExtendedPublicKeyRecord, InputRecord, Label, OutputRecord, PublicKeyRecord,
        TransactionRecord,
    };
    use bitcoin::Address;
    use bitcoin::Network;
    use bitcoin::address::NetworkUnchecked;
    use bitcoin::bip32::Xpub;
    use bitcoin::{OutPoint, PublicKey, Txid};
    use std::matches;
    use std::str::FromStr;

    use super::*;

    /// An unfunded wallet, for cases that need `add_label` to reject a target —
    /// this wallet has never seen any transaction or derived-and-funded address.
    fn test_wallet() -> Wallet {
        let external_desc =
            "wpkh(0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798)";
        let internal_desc =
            "wpkh(03a0434d9e47f3c86235477c7b1ae6ae5d3442d49b1943c2b752a68e2a47e247c7)";

        Wallet::create(external_desc, internal_desc)
            .network(Network::Testnet)
            .create_wallet_no_persist()
            .expect("Failed to create wallet")
    }

    /// A wallet funded via a real, wallet-known transaction: `get_funded_wallet`
    /// creates `tx0` (76_000 sats received) then `tx1`, which spends that output
    /// and creates a 50_000 sat change output plus a 25_000 sat foreign payment.
    /// This gives, from one call: a real `Txid` (returned), a real spent output
    /// (a valid `Input` target), and a real unspent output (a valid `Output`
    /// target) — everything `add_label`'s wallet-membership validation needs.
    fn funded_test_wallet() -> (Wallet, Txid) {
        let (desc, change_desc) = get_test_wpkh_and_change_desc();
        get_funded_wallet(desc, change_desc)
    }

    /// Pulls the (unspent, spent) outpoints out of a wallet produced by
    /// `funded_test_wallet`, without assuming which of `tx0`/`tx1`'s outputs
    /// ends up in which position.
    fn unspent_and_spent_outpoints(wallet: &Wallet) -> (OutPoint, OutPoint) {
        let unspent_outpoint = wallet
            .list_unspent()
            .next()
            .expect("funded wallet should have one unspent output")
            .outpoint;

        let spent_outpoint = wallet
            .list_output()
            .map(|o| o.outpoint)
            .find(|op| *op != unspent_outpoint)
            .expect("funded wallet should also have a spent output (tx1's input)");

        (unspent_outpoint, spent_outpoint)
    }

    fn as_unchecked(address: Address) -> Address<NetworkUnchecked> {
        address
            .to_string()
            .parse()
            .expect("Wallet derived address should be converted to Network Unchecked")
    }

    #[test]
    fn test_add_label_variant_mapping() {
        let (mut wallet, funding_txid) = funded_test_wallet();

        let (unspent_outpoint, spent_outpoint) = unspent_and_spent_outpoints(&wallet);
        let owned_address = wallet.reveal_next_address(KeychainKind::External).address;

        let dummy_pubkey = PublicKey::from_str(
            "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798",
        )
        .unwrap();

        let dummy_xpub = Xpub::from_str("xpub661MyMwAqRbcFtXgS5sYJABqqG9YLmC4Q1Rdap9gSE8NqtwybGhePY2gZ29ESFjqJoCu1Rupje8YtGqsefD265TMg7usUDFdp6W1EGMcet8").unwrap();

        let mut changeset = LabelChangeset::new();

        let mut labelled_wallet = LabelledWallet {
            wallet: &mut wallet,
            labels: &mut changeset,
        };

        let transaction_label = labelled_wallet
            .add_label(funding_txid, "Payment for Machinery")
            .expect("Failed to add transaction label");

        let address_label = labelled_wallet
            .add_label(as_unchecked(owned_address.clone()), "Employee address")
            .expect("Failed to add address label");

        let pubkey_label = labelled_wallet
            .add_label(dummy_pubkey, "My wallet's public key")
            .expect("Failed to add address label");

        let input_label = labelled_wallet
            .add_label(InputTarget(spent_outpoint), "My transaction's input")
            .expect("Failed to add address label");

        let output_label = labelled_wallet
            .add_label(OutputTarget(unspent_outpoint), "My transaction's Output")
            .expect("Failed to add address label");

        let xpub_label = labelled_wallet
            .add_label(dummy_xpub, "My wallet's extended public key")
            .expect("Failed to add address label");

        assert!(matches!(
            transaction_label,
            Label::Transaction(TransactionRecord {
                ref_: _,
                label: Some(_),
                origin: _,
            })
        ));

        assert!(matches!(
            address_label,
            Label::Address(AddressRecord {
                ref_: _,
                label: Some(_)
            })
        ));

        assert!(matches!(
            pubkey_label,
            Label::PublicKey(PublicKeyRecord {
                ref_: _,
                label: Some(_),
            })
        ));

        assert!(matches!(
            input_label,
            Label::Input(InputRecord {
                ref_: _,
                label: Some(_),
            })
        ));

        assert!(matches!(
            output_label,
            Label::Output(OutputRecord {
                ref_: _,
                label: Some(_),
                spendable: true,
            })
        ));

        assert!(matches!(
            xpub_label,
            Label::ExtendedPublicKey(ExtendedPublicKeyRecord {
                ref_: _,
                label: Some(_),
            })
        ));
    }

    #[test]
    fn test_add_label_rejects_address_not_owned_by_wallet() {
        let mut wallet = test_wallet();
        let mut changeset = LabelChangeset::new();
        let mut labelled_wallet = LabelledWallet {
            wallet: &mut wallet,
            labels: &mut changeset,
        };

        let unowned_address = bitcoin::Address::from_str("mkHS9ne12qx9pS9VojpwU5xtRd4T7X7ZUt")
            .expect("failed to parse address");

        let result = labelled_wallet.add_label(unowned_address, "Not for this wallet");

        assert!(matches!(result, Err(Error::Validation(_))));
    }

    #[test]
    fn test_add_label_rejects_unknown_outpoint() {
        let mut wallet = test_wallet();
        let mut changeset = LabelChangeset::new();
        let mut labelled_wallet = LabelledWallet {
            wallet: &mut wallet,
            labels: &mut changeset,
        };

        let dummy_txid =
            Txid::from_str("0000000000000000000000000000000000000000000000000000000000000000")
                .unwrap();
        let unknown_outpoint = OutPoint::new(dummy_txid, 0);

        let output_result =
            labelled_wallet.add_label(OutputTarget(unknown_outpoint), "Not a real UTXO");
        let input_result =
            labelled_wallet.add_label(InputTarget(unknown_outpoint), "Not a real input");

        assert!(matches!(output_result, Err(Error::Validation(_))));
        assert!(matches!(input_result, Err(Error::Validation(_))));
    }

    #[test]
    fn test_add_label_preserves_existing_spendable_state() {
        let (mut wallet, _funding_txid) = funded_test_wallet();
        let (unspent_outpoint, _spent_outpoint) = unspent_and_spent_outpoints(&wallet);

        let mut changeset = LabelChangeset::new();

        changeset.insert(Label::Output(bip329::OutputRecord {
            ref_: unspent_outpoint,
            label: Some("Dummy Label".to_string()),
            spendable: false,
        }));

        let mut labelled_wallet = LabelledWallet {
            wallet: &mut wallet,
            labels: &mut changeset,
        };

        let relabelled_output = labelled_wallet
            .add_label(OutputTarget(unspent_outpoint), "My transaction's Output")
            .expect("Failed to add address label");

        assert!(matches!(
            relabelled_output,
            Label::Output(OutputRecord {
                spendable: false,
                ..
            })
        ))
    }

    #[test]
    fn test_add_label_defaults_new_outputs_to_spendadle_true() {
        let (mut wallet, _funding_txid) = funded_test_wallet();
        let (unspent_outpoint, _spent_outpoint) = unspent_and_spent_outpoints(&wallet);

        let mut changeset = LabelChangeset::new();

        let mut labelled_wallet = LabelledWallet {
            wallet: &mut wallet,
            labels: &mut changeset,
        };

        let new_output = labelled_wallet
            .add_label(OutputTarget(unspent_outpoint), "New Output")
            .expect("Failed to add address label");

        assert!(matches!(
            new_output,
            Label::Output(OutputRecord {
                spendable: true,
                ..
            })
        ))
    }

    #[test]
    fn test_mock_persister_captures_only_staged_labels() {
        let (mut wallet, funding_txid) = funded_test_wallet();

        let mut changeset = LabelChangeset::new();

        let mut labelled_wallet = LabelledWallet {
            wallet: &mut wallet,
            labels: &mut changeset,
        };

        use std::convert::Infallible;
        pub struct MockPersister {
            pub received_changesets: Vec<LabelChangeset>,
        }

        impl LabelPersister for MockPersister {
            type Error = Infallible;

            fn read_labels(&self) -> Result<LabelChangeset, Self::Error> {
                Ok(LabelChangeset::default())
            }

            fn append_changeset(&mut self, changeset: &LabelChangeset) -> Result<(), Self::Error> {
                self.received_changesets.push(changeset.clone());
                Ok(())
            }
        }

        let mut mock_persister = MockPersister {
            received_changesets: vec![],
        };

        assert_eq!(mock_persister.received_changesets.len(), 0);

        let transaction_label = labelled_wallet
            .add_label(funding_txid, "Payment for Machinery")
            .expect("Failed to add transaction label");

        labelled_wallet
            .persist(&mut mock_persister)
            .expect("first persist should succeed");

        assert_eq!(mock_persister.received_changesets.len(), 1);
        assert_eq!(mock_persister.received_changesets[0].len(), 1);

        labelled_wallet
            .persist(&mut mock_persister)
            .expect("second, no-op persist should succeed");

        assert_eq!(
            mock_persister.received_changesets.len(),
            1,
            "persist() must not re-send unchanged labels"
        );

        assert!(matches!(
            transaction_label,
            Label::Transaction(TransactionRecord {
                ref_: _,
                label: Some(_),
                origin: _,
            })
        ));

        let persisted_changeset = &mock_persister.received_changesets[0];

        assert_eq!(
            persisted_changeset.get(&transaction_label.ref_()),
            Some(&transaction_label)
        );
    }

    #[test]
    fn test_wallet_io_delegation_roundtrip() {
        let mut source_wallet = test_wallet();
        let owned_address = source_wallet
            .reveal_next_address(KeychainKind::External)
            .address;

        let mut source_changeset = LabelChangeset::new();

        let mut source_labelled_wallet = LabelledWallet {
            wallet: &mut source_wallet,
            labels: &mut source_changeset,
        };

        assert_eq!(source_labelled_wallet.labels.len(), 0);

        let address_label = source_labelled_wallet
            .add_label(as_unchecked(owned_address.clone()), "Employee address")
            .expect("Failed to add address label");

        assert_eq!(source_labelled_wallet.labels.len(), 1);

        let mut buffer = Vec::new();

        source_labelled_wallet
            .export_labels(&mut buffer)
            .expect("Failed to export labels");

        assert!(
            !buffer.is_empty(),
            "The exported buffer should contain data"
        );

        let mut dest_wallet = test_wallet();
        let mut dest_changeset = LabelChangeset::new();
        let mut dest_labelled_wallet = LabelledWallet {
            wallet: &mut dest_wallet,
            labels: &mut dest_changeset,
        };

        assert_eq!(dest_labelled_wallet.labels.len(), 0);

        let reader = std::io::Cursor::new(buffer);

        dest_labelled_wallet
            .import_labels(reader, MergeStrategy::Overwrite)
            .expect("Failed to import labels");

        assert_eq!(dest_labelled_wallet.labels.len(), 1);

        assert_eq!(
            dest_labelled_wallet.labels.get(&address_label.ref_()),
            Some(&address_label)
        );
    }
}
