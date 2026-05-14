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
