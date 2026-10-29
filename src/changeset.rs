use bip329::{Label, LabelRef};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

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
    // Refs that have not been persisted to the wallet's database
    #[serde(skip)]
    staged: BTreeSet<LabelRef>,
}

impl LabelChangeset {
    /// Creates a new, empty `LabelChangeset`.
    pub fn new() -> Self {
        Self {
            labels: BTreeMap::new(),
            staged: BTreeSet::new(),
        }
    }

    /// Inserts a new label into the changeset making its ref staged.
    /// If a label with the same reference already exists, it is overwritten.
    pub fn insert(&mut self, label: Label) {
        let target = label.ref_();
        self.staged.insert(target.clone());
        self.labels.insert(target, label);
    }

    /// Removes the label associated with the given target reference, returning it if it existed.
    pub fn remove(&mut self, target: &LabelRef) -> Option<Label> {
        self.staged.insert(target.clone());
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
    ///
    /// Every label that ends up changing this changeset is staged, so that a following
    /// [`LabelledWallet::persist`](crate::LabelledWallet::persist) writes it.
    pub fn merge(&mut self, incoming: LabelChangeset, strategy: MergeStrategy) {
        for (target, incoming_label) in incoming.labels {
            match strategy {
                MergeStrategy::Overwrite => {
                    self.staged.insert(target.clone());
                    self.labels.insert(target, incoming_label);
                }
                MergeStrategy::KeepExisting => {
                    if let std::collections::btree_map::Entry::Vacant(entry) =
                        self.labels.entry(target.clone())
                    {
                        entry.insert(incoming_label);
                        self.staged.insert(target);
                    }
                }
            }
        }
    }

    /// Returns the unpersisted diff of the changeset.
    pub fn diff(&self) -> LabelChangeset {
        let mut d = LabelChangeset::new();

        for target in &self.staged {
            if let Some(label) = self.labels.get(target) {
                d.labels.insert(target.clone(), label.clone());
            }
        }
        d
    }

    /// Clears the staged label refs
    pub fn clear_staged(&mut self) {
        self.staged.clear();
    }

    /// Checks and returns true if there are changes staged
    pub fn has_staged_changes(&self) -> bool {
        !self.staged.is_empty()
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

    #[test]
    fn test_diff_only_contains_staged_entries_and_clears_on_demand() {
        let mut changeset = LabelChangeset::new();

        let first_txid =
            Txid::from_str("0000000000000000000000000000000000000000000000000000000000000000")
                .unwrap();
        let second_txid =
            Txid::from_str("0000000000000000000000000000000000000000000000000000000000000110")
                .unwrap();

        let first_label = Label::Transaction(TransactionRecord {
            ref_: first_txid,
            label: Some("First Transaction".to_string()),
            origin: None,
        });
        let second_label = Label::Transaction(TransactionRecord {
            ref_: second_txid,
            label: Some("Second Transaction".to_string()),
            origin: None,
        });

        changeset.insert(first_label.clone());
        assert!(changeset.has_staged_changes());

        let first_diff = changeset.diff();

        assert_eq!(first_diff.labels.len(), 1);

        assert_eq!(first_diff.get(&first_label.ref_()), Some(&first_label));

        changeset.clear_staged();
        assert!(!changeset.has_staged_changes());

        let empty_diff = changeset.diff();
        assert!(empty_diff.is_empty());
        assert_eq!(first_diff.labels.len(), 1);

        changeset.insert(second_label.clone());
        assert!(changeset.has_staged_changes());
        let second_diff = changeset.diff();
        assert_eq!(second_diff.labels.len(), 1);
        assert_eq!(second_diff.get(&second_label.ref_()), Some(&second_label));
        assert_eq!(second_diff.get(&first_label.ref_()), None);
    }

    fn tx_label(txid: &str, text: &str) -> Label {
        Label::Transaction(TransactionRecord {
            ref_: Txid::from_str(txid).unwrap(),
            label: Some(text.to_string()),
            origin: None,
        })
    }

    const TXID: &str = "5fe2524f0c46bb12767d1865114031cd631578ac05422a767c0edf23f5c0aeaa";

    fn incoming(text: &str) -> LabelChangeset {
        let mut incoming = LabelChangeset::new();
        incoming.insert(tx_label(TXID, text));
        incoming
    }

    #[test]
    fn test_merge_overwrite_stages_merged_labels() {
        let mut changeset = LabelChangeset::new();

        changeset.merge(incoming("new"), MergeStrategy::Overwrite);

        assert!(changeset.has_staged_changes());
        assert_eq!(changeset.diff().len(), 1);
    }

    #[test]
    fn test_merge_keep_existing_stages_only_new_labels() {
        let mut changeset = LabelChangeset::new();

        changeset.merge(incoming("first"), MergeStrategy::KeepExisting);
        assert_eq!(changeset.diff().len(), 1);

        changeset.clear_staged();
        changeset.merge(incoming("second"), MergeStrategy::KeepExisting);

        assert!(!changeset.has_staged_changes());
        assert_eq!(
            changeset.iter().next().and_then(|l| l.label()),
            Some("first")
        );
    }
}
