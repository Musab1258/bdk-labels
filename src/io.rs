use crate::changeset::LabelChangeset;
use crate::error::Error;
use bip329::Label;
use std::io::{BufRead, Write};

/// Serializes and writes a `LabelChangeset` to a standard output stream.
///
/// The output is formatted as a BIP-329 compliant JSONL (JSON Lines) string.
/// Because `LabelChangeset` utilizes a `BTreeMap`, the resulting lines are
/// deterministically ordered by their reference key.
pub fn export<W: Write>(labels: &LabelChangeset, mut writer: W) -> Result<(), Error> {
    for label in labels.iter() {
        let line = serde_json::to_string(label)?;
        writeln!(writer, "{}", line)?;
    }
    Ok(())
}

/// Reads and deserializes a BIP-329 JSONL stream into a new `LabelChangeset`.
///
/// Blank lines and trailing newlines are safely ignored during the import process.
pub fn import<R: BufRead>(reader: R) -> Result<LabelChangeset, Error> {
    let mut imported_labels = LabelChangeset::new();
    for line_result in reader.lines() {
        let line_str = line_result?;

        if line_str.trim().is_empty() {
            continue;
        }

        let line: Label = serde_json::from_str(&line_str)?;
        imported_labels.insert(line);
    }
    Ok(imported_labels)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bip329::{AddressRecord, Label};
    use bitcoin::Address;
    use std::str::FromStr;

    #[test]
    fn test_round_trip_export_import() {
        let mut changeset = LabelChangeset::new();

        let dummy_address =
            Address::from_str("bc1p0dq0tzg2r780hldthn5mrznmpxsxc0jux5f20fwj0z3wqxxk6fpqm7q0va")
                .expect("Failed to parse address");

        let dummy_label = Label::Address(AddressRecord {
            ref_: dummy_address,
            label: Some("Heavy Machinery".to_string()),
        });

        changeset.insert(dummy_label.clone());

        let mut buffer = Vec::new();

        export(&changeset, &mut buffer).expect("Failed to export changeset");

        let reader = std::io::Cursor::new(buffer);

        let imported_file = import(reader).expect("Failed to import changeset");

        assert_eq!(imported_file.get(&dummy_label.ref_()), Some(&dummy_label));
    }

    #[test]
    fn test_empty_export_import_states() {
        let changeset = LabelChangeset::new();

        let mut buffer = Vec::new();

        export(&changeset, &mut buffer).expect("Failed to export changeset");

        assert_eq!(buffer.len(), 0);

        let reader = std::io::Cursor::new(buffer);

        let imported_file = import(reader).expect("Failed to import changeset");

        assert_eq!(imported_file.len(), 0);
    }

    #[test]
    fn test_trailing_newlines_and_blank_lines() {
        // 1. A raw string simulating a JSONL file with extra blank lines at the end
        //{"type": "tx", "ref": "...", "label": "..."}
        let messy_jsonl = r#"{"type": "tx","ref":"0000000000000000000000000000000000000000000000000000000000000000","label":"Machinery","origin":null}


        "#;

        // 2. Wrap it in a Cursor so it implements BufRead
        let reader = std::io::Cursor::new(messy_jsonl.as_bytes());

        // 3. Attempt to import
        let imported_changeset = import(reader).expect("Failed to import messy JSONL");

        assert_eq!(imported_changeset.len(), 1);
    }

    #[test]
    fn test_fail_fast_on_corrupted_jsonl() {
        let corrupted_jsonl = r#"{"type": "tx", "ref": "0000000000000000000000000000000000000000000000000000000000000000, "label": "Oops"}"#;

        let reader = std::io::Cursor::new(corrupted_jsonl.as_bytes());

        let imported_changeset = import(reader);

        assert!(imported_changeset.is_err());
    }
}
