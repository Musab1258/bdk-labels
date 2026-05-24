use crate::{Bip329, LabelTarget};
use bdk_wallet::Wallet;

use crate::Error;
use crate::changeset::{LabelChangeset, MergeStrategy};
use crate::persist::LabelPersister;
use crate::{export, import};
use bip329::Label;
use std::io::{BufRead, Write};

pub struct LabelledWallet<'a> {
    pub wallet: &'a mut Wallet,

    pub labels: &'a mut LabelChangeset,
}

impl Bip329 for LabelledWallet<'_> {
    fn add_label(
        &mut self,

        target: impl Into<LabelTarget>,

        label_text: impl Into<String>,
    ) -> Result<Label, Error> {
        let new_label = match target.into() {
            LabelTarget::Txid(txid) => Label::Transaction(bip329::TransactionRecord {
                ref_: txid,
                label: Some(label_text.into()),
                origin: None,
            }),
            LabelTarget::Address(addr) => Label::Address(bip329::AddressRecord {
                ref_: addr,
                label: Some(label_text.into()),
            }),
            LabelTarget::PublicKey(pk) => Label::PublicKey(bip329::PublicKeyRecord {
                ref_: pk,
                label: Some(label_text.into()),
            }),
            LabelTarget::Input(outpoint) => Label::Input(bip329::InputRecord {
                ref_: outpoint,
                label: Some(label_text.into()),
            }),
            LabelTarget::Output(outpoint) => Label::Output(bip329::OutputRecord {
                ref_: outpoint,
                label: Some(label_text.into()),
                spendable: false,
            }),
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
    pub fn persist<P: LabelPersister>(&mut self, persister: &mut P) -> Result<(), Error> {
        persister
            .append_changeset(self.labels)
            .map_err(|e| Error::Custom(Box::new(e)))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::changeset::LabelChangeset;
    use crate::{InputTarget, OutputTarget};
    use bdk_wallet::Wallet;
    use bip329::{
        AddressRecord, ExtendedPublicKeyRecord, InputRecord, Label, OutputRecord, PublicKeyRecord,
        TransactionRecord,
    };
    use bitcoin::Network;
    use bitcoin::bip32::Xpub;
    use bitcoin::{Address, OutPoint, PublicKey, Txid};
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_add_label_variant_mapping() {
        let external_desc =
            "wpkh(0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798)";
        let internal_desc =
            "wpkh(03a0434d9e47f3c86235477c7b1ae6ae5d3442d49b1943c2b752a68e2a47e247c7)";

        let mut wallet = Wallet::create(external_desc, internal_desc)
            .network(Network::Testnet)
            .create_wallet_no_persist()
            .expect("Failed to create wallet");

        let mut changeset = LabelChangeset::new();

        let mut labelled_wallet = LabelledWallet {
            wallet: &mut wallet,
            labels: &mut changeset,
        };

        let dummy_txid =
            Txid::from_str("0000000000000000000000000000000000000000000000000000000000000000")
                .unwrap();

        let dummy_address = Address::from_str("mkHS9ne12qx9pS9VojpwU5xtRd4T7X7ZUt")
            .expect("Failed to parse address");

        let dummy_outpoint = OutPoint::new(dummy_txid, 0);

        let dummy_pubkey = PublicKey::from_str(
            "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798",
        )
        .unwrap();

        let dummy_xpub = Xpub::from_str("xpub661MyMwAqRbcFtXgS5sYJABqqG9YLmC4Q1Rdap9gSE8NqtwybGhePY2gZ29ESFjqJoCu1Rupje8YtGqsefD265TMg7usUDFdp6W1EGMcet8").unwrap();

        let transaction_label = labelled_wallet
            .add_label(dummy_txid, "Payment for Machinery")
            .expect("Failed to add transaction label");

        let address_label = labelled_wallet
            .add_label(dummy_address, "Employee address")
            .expect("Failed to add address label");

        let pubkey_label = labelled_wallet
            .add_label(dummy_pubkey, "My wallet's public key")
            .expect("Failed to add address label");

        let input_label = labelled_wallet
            .add_label(InputTarget(dummy_outpoint), "My transaction's input")
            .expect("Failed to add address label");

        let output_label = labelled_wallet
            .add_label(OutputTarget(dummy_outpoint), "My transaction's Output")
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
                spendable: false,
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
}
