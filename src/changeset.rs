use bip329::{Label, LabelRef};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeStrategy {
    Overwrite,
    KeepExisting,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct LabelChangeset {
    labels: HashMap<LabelRef, Label>,
}

impl LabelChangeset {
    pub fn new() -> Self {
        Self {
            labels: HashMap::new(),
        }
    }

    pub fn insert(&mut self, label: Label) {
        self.labels.insert(label.ref_(), label);
    }

    pub fn remove(&mut self, target: &LabelRef) -> Option<Label> {
        self.labels.remove(target)
    }

    pub fn get(&self, target: &LabelRef) -> Option<&Label> {
        self.labels.get(target)
    }

    pub fn is_empty(&self) -> bool {
        self.labels.is_empty()
    }

    pub fn len(&self) -> usize {
        self.labels.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Label> {
        self.labels.values()
    }

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
    use bip329::{Label, TransactionRecord};
    use bitcoin::Txid;
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
}
