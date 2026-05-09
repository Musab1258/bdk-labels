use bip329::{Label, LabelRef};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LabelChangeset {
    labels: HashMap<LabelRef, Label>,
}

impl LabelChangeset {
    pub fn insert(&mut self, label: Label) {
        self.labels.insert(label.ref_(), label);
    }

    pub fn is_empty(&self) -> bool {
        self.labels.is_empty()
    }

    pub fn append(&mut self, other: LabelChangeset) {
        self.labels.extend(other.labels);
    }
}
