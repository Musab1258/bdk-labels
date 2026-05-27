use bip329::{Label, LabelRef};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Defines the strategy for resolving conflicts when merging two label sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeStrategy {
    /// The incoming label will overwrite the existing label if they share the same reference.
    Overwrite,
    /// The existing label is preserved; the incoming label is ignored if they share the same reference.
    KeepExisting,
}

/// An in-memory, deterministic collection of BIP-329 wallet labels.
///
/// Backed by a `BTreeMap`, this structure ensures $O(\log n)$ deduplication and
/// guarantees that labels are deterministically sorted by their reference key.
/// This prevents noisy diffs when exporting to version-controlled JSONL files.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct LabelChangeset {
    labels: BTreeMap<LabelRef, Label>,
}

impl LabelChangeset {
    /// Creates a new, empty `LabelChangeset`.
    pub fn new() -> Self {
        Self {
            labels: BTreeMap::new(),
        }
    }

    /// Inserts a new label into the changeset.
    /// If a label with the same reference already exists, it is overwritten.
    pub fn insert(&mut self, label: Label) {
        self.labels.insert(label.ref_(), label);
    }

    /// Removes the label associated with the given target reference, returning it if it existed.
    pub fn remove(&mut self, target: &LabelRef) -> Option<Label> {
        self.labels.remove(target)
    }

    /// Retrieves a reference to the label associated with the given target.
    pub fn get(&self, target: &LabelRef) -> Option<&Label> {
        self.labels.get(target)
    }

    /// Returns `true` if the changeset contains no labels.
    pub fn is_empty(&self) -> bool {
        self.labels.is_empty()
    }

    /// Returns the total number of labels in the changeset.
    pub fn len(&self) -> usize {
        self.labels.len()
    }

    /// Returns an iterator over the labels, yielded in deterministic order based on their reference.
    pub fn iter(&self) -> impl Iterator<Item = &Label> {
        self.labels.values()
    }

    /// Merges an incoming `LabelChangeset` into the current one, resolving conflicts based on the provided `MergeStrategy`.
    pub fn merge(&mut self, incoming: LabelChangeset, strategy: MergeStrategy) {
        for (_, incoming_label) in incoming.labels {
            let target = incoming_label.ref_();
            match strategy {
                MergeStrategy::Overwrite => {
                    self.labels.insert(target, incoming_label);
                }
                MergeStrategy::KeepExisting => {
                    self.labels.entry(target).or_insert(incoming_label);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bip329::{AddressRecord, Label, TransactionRecord};
    use bitcoin::{Address, Txid};
    use std::str::FromStr;

    #[test]
    fn test_basic_crud_operations() {
        let mut changeset = LabelChangeset::new();
        assert!(changeset.is_empty(), "Expected new changeset to be empty");
        assert_eq!(changeset.len(), 0);

        let dummy_txid =
            Txid::from_str("0000000000000000000000000000000000000000000000000000000000000000")
                .unwrap();

        let dummy_label = Label::Transaction(TransactionRecord {
            ref_: dummy_txid,
            label: Some("Machinery".to_string()),
            origin: None,
        });

        changeset.insert(dummy_label.clone());

        assert_eq!(changeset.len(), 1);
        assert_eq!(changeset.get(&dummy_label.ref_()), Some(&dummy_label));

        let updated_label = Label::Transaction(TransactionRecord {
            ref_: dummy_txid,
            label: Some("Heavy Machinery".to_string()),
            origin: None,
        });
        changeset.insert(updated_label.clone());

        assert_eq!(changeset.len(), 1);
        assert_eq!(changeset.get(&dummy_label.ref_()), Some(&updated_label));

        let removed_label = changeset.remove(&dummy_label.ref_());

        assert_eq!(removed_label, Some(updated_label));
        assert!(changeset.is_empty());
        assert_eq!(changeset.get(&dummy_label.ref_()), None);
    }

    #[test]
    fn test_implicit_deduplication() {
        let mut changeset = LabelChangeset::new();

        let dummy_txid =
            Txid::from_str("0000000000000000000000000000000000000000000000000000000000000000")
                .unwrap();

        let dummy_label = Label::Transaction(TransactionRecord {
            ref_: dummy_txid,
            label: Some("Machinery".to_string()),
            origin: None,
        });

        changeset.insert(dummy_label.clone());
        changeset.insert(dummy_label.clone());
        changeset.insert(dummy_label.clone());

        assert_eq!(changeset.len(), 1);
    }

    #[test]
    fn test_merge_strategy_overwrite() {
        let mut base_changeset = LabelChangeset::new();
        let mut incoming_changeset = LabelChangeset::new();

        let dummy_txid =
            Txid::from_str("0000000000000000000000000000000000000000000000000000000000000000")
                .unwrap();

        let dummy_label = Label::Transaction(TransactionRecord {
            ref_: dummy_txid,
            label: Some("Machinery".to_string()),
            origin: None,
        });

        let another_dummy_label = Label::Transaction(TransactionRecord {
            ref_: dummy_txid,
            label: Some("Heavy Machinery".to_string()),
            origin: None,
        });

        base_changeset.insert(dummy_label.clone());

        incoming_changeset.insert(another_dummy_label.clone());

        base_changeset.merge(incoming_changeset, MergeStrategy::Overwrite);

        assert_eq!(
            base_changeset.get(&dummy_label.ref_()),
            Some(&another_dummy_label)
        );
    }

    #[test]
    fn test_merge_strategy_keep_existing() {
        let mut base_changeset = LabelChangeset::new();
        let mut incoming_changeset = LabelChangeset::new();

        let dummy_txid =
            Txid::from_str("0000000000000000000000000000000000000000000000000000000000000000")
                .unwrap();

        let dummy_label = Label::Transaction(TransactionRecord {
            ref_: dummy_txid,
            label: Some("Machinery".to_string()),
            origin: None,
        });

        let another_dummy_label = Label::Transaction(TransactionRecord {
            ref_: dummy_txid,
            label: Some("Heavy Machinery".to_string()),
            origin: None,
        });

        base_changeset.insert(dummy_label.clone());

        incoming_changeset.insert(another_dummy_label.clone());

        base_changeset.merge(incoming_changeset, MergeStrategy::KeepExisting);

        assert_eq!(base_changeset.get(&dummy_label.ref_()), Some(&dummy_label));
    }

    #[test]
    fn test_non_overlapping_merges() {
        let mut base_changeset = LabelChangeset::new();
        let mut incoming_changeset = LabelChangeset::new();

        let dummy_txid =
            Txid::from_str("0000000000000000000000000000000000000000000000000000000000000000")
                .unwrap();

        let another_dummy_txid =
            Txid::from_str("0000000000000000000000000000000000000000000000000000000000000110")
                .unwrap();

        let dummy_label = Label::Transaction(TransactionRecord {
            ref_: dummy_txid,
            label: Some("Machinery".to_string()),
            origin: None,
        });

        let another_dummy_label = Label::Transaction(TransactionRecord {
            ref_: another_dummy_txid,
            label: Some("Heavy Machinery".to_string()),
            origin: None,
        });

        base_changeset.insert(dummy_label.clone());

        incoming_changeset.insert(another_dummy_label.clone());

        base_changeset.merge(incoming_changeset, MergeStrategy::Overwrite);

        assert_eq!(base_changeset.len(), 2);
        assert_eq!(base_changeset.get(&dummy_label.ref_()), Some(&dummy_label));
        assert_eq!(
            base_changeset.get(&another_dummy_label.ref_()),
            Some(&another_dummy_label)
        );
    }

    #[test]
    fn test_empty_merges() {
        let mut populated_base_changeset = LabelChangeset::new();
        let empty_incoming_changeset = LabelChangeset::new();

        let dummy_txid =
            Txid::from_str("0000000000000000000000000000000000000000000000000000000000000000")
                .unwrap();

        let dummy_label = Label::Transaction(TransactionRecord {
            ref_: dummy_txid,
            label: Some("Machinery".to_string()),
            origin: None,
        });

        populated_base_changeset.insert(dummy_label.clone());

        populated_base_changeset.merge(empty_incoming_changeset, MergeStrategy::Overwrite);

        assert_eq!(populated_base_changeset.len(), 1);
        assert_eq!(
            populated_base_changeset.get(&dummy_label.ref_()),
            Some(&dummy_label)
        );

        let mut empty_base_changeset = LabelChangeset::new();
        let mut populated_incoming_changeset = LabelChangeset::new();

        let dummy_address =
            Address::from_str("bc1p0dq0tzg2r780hldthn5mrznmpxsxc0jux5f20fwj0z3wqxxk6fpqm7q0va")
                .expect("Failed to parse address");

        let another_dummy_label = Label::Address(AddressRecord {
            ref_: dummy_address,
            label: Some("Heavy Machinery".to_string()),
        });

        populated_incoming_changeset.insert(another_dummy_label.clone());

        empty_base_changeset.merge(populated_incoming_changeset, MergeStrategy::Overwrite);

        assert_eq!(empty_base_changeset.len(), 1);
        assert_eq!(
            empty_base_changeset.get(&another_dummy_label.ref_()),
            Some(&another_dummy_label)
        );
    }
}
