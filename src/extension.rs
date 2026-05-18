use crate::Bip329;
use bdk_wallet::Wallet;

use crate::Error;
use crate::changeset::{LabelChangeset, MergeStrategy};
use crate::persist::LabelPersister;
use crate::{export, import};
use bip329::{Label, LabelRef};
use std::io::{BufRead, Write};

pub struct LabelledWallet<'a> {
    pub wallet: &'a mut Wallet,

    pub labels: &'a mut LabelChangeset,
}

impl<'a> Bip329 for LabelledWallet<'a> {
    fn add_label(
        &mut self,

        target: impl Into<LabelRef>,

        label_text: impl Into<String>,
    ) -> Result<Label, Error> {
        let new_label = match target.into() {
            LabelRef::Txid(txid) => Label::Transaction(bip329::TransactionRecord {
                ref_: txid,
                label: Some(label_text.into()),
                origin: None,
            }),
            LabelRef::Address(addr) => Label::Address(bip329::AddressRecord {
                ref_: addr,
                label: Some(label_text.into()),
            }),
            LabelRef::PublicKey(pk) => Label::PublicKey(bip329::PublicKeyRecord {
                ref_: pk,
                label: Some(label_text.into()),
            }),
            LabelRef::Input(outpoint) => Label::Input(bip329::InputRecord {
                ref_: outpoint,
                label: Some(label_text.into()),
            }),
            LabelRef::Output(outpoint) => Label::Output(bip329::OutputRecord {
                ref_: outpoint,
                label: Some(label_text.into()),
                spendable: false,
            }),
            LabelRef::Xpub(xpub) => Label::ExtendedPublicKey(bip329::ExtendedPublicKeyRecord {
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
        let _ = export(self.labels, writer);
        Ok(())
    }
}

impl<'a> LabelledWallet<'a> {
    pub fn persist<P: LabelPersister>(&mut self, persister: &mut P) -> Result<(), Error> {
        persister.append_changeset(self.labels)?;
        Ok(())
    }
}
