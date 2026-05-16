use crate::Changeset::LabelChangeset;
use crate::error::Error;
use std::io::{BufRead, Write};

pub fn export<W: Write>(labels: &LabelChangeset, mut writer: W) -> Result<(), Error> {
    for label in labels.iter() {
        let line = serde_json::to_string(label)?;
        writeln!(writer, "{}", line)?;
    }
    Ok(())
}

pub fn import<R: BufRead>(reader: R) -> Result<LabelChangeset, Error> {
    let mut imported_labels = LabelChangeset::new();
    for line_result in reader.lines() {
        let line = serde_json::from_str(&line_result?)?;
        imported_labels.insert(line);
    }
    Ok(imported_labels)
}
