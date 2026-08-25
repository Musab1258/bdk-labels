use bdk_labels::{
    Bip329, Error, InputTarget, LabelChangeset, LabelPersister, LabelledWallet, MergeStrategy,
    OutputTarget,
};
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
use std::convert::Infallible;
use std::fmt::Display;
use std::str::FromStr;
pub struct IntegrationMockDB {
    pub received_changesets: Vec<LabelChangeset>,
}

impl LabelPersister for IntegrationMockDB {
    type Error = Infallible;

    fn read_labels(&self) -> Result<LabelChangeset, Self::Error> {
        Ok(LabelChangeset::default())
    }

    fn append_changeset(&mut self, changeset: &LabelChangeset) -> Result<(), Self::Error> {
        self.received_changesets.push(changeset.clone());
        Ok(())
    }
}

#[derive(Debug)]
pub struct MockDbError;

impl Display for MockDbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Mock database write failed")
    }
}

impl std::error::Error for MockDbError {}

pub struct FailingMockDb;

impl LabelPersister for FailingMockDb {
    type Error = MockDbError;

    fn read_labels(&self) -> Result<LabelChangeset, Self::Error> {
        Ok(LabelChangeset::default())
    }

    fn append_changeset(&mut self, _changeset: &LabelChangeset) -> Result<(), Self::Error> {
        Err(MockDbError)
    }
}

fn funded_test_wallet() -> (Wallet, Txid) {
    let (desc, change_desc) = get_test_wpkh_and_change_desc();
    get_funded_wallet(desc, change_desc)
}

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

// take a wallet from creation, through labeling, to final database persistence and verification?
#[test]
fn test_full_labelling_lifecycle() {
    let mut integration_mock_db = IntegrationMockDB {
        received_changesets: vec![],
    };

    assert_eq!(integration_mock_db.received_changesets.len(), 0);

    let (mut test_wallet, funding_txid) = funded_test_wallet();

    let (unspent_outpoint, spent_outpoint) = unspent_and_spent_outpoints(&test_wallet);

    let dummy_address = test_wallet
        .reveal_next_address(KeychainKind::External)
        .address;

    let mut test_changeset = LabelChangeset::new();

    let mut test_labelled_wallet = LabelledWallet {
        wallet: &mut test_wallet,
        labels: &mut test_changeset,
    };

    assert_eq!(test_labelled_wallet.labels.len(), 0);

    let dummy_pubkey =
        PublicKey::from_str("0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798")
            .unwrap();

    let dummy_xpub = Xpub::from_str("xpub661MyMwAqRbcFtXgS5sYJABqqG9YLmC4Q1Rdap9gSE8NqtwybGhePY2gZ29ESFjqJoCu1Rupje8YtGqsefD265TMg7usUDFdp6W1EGMcet8").unwrap();

    let transaction_label = test_labelled_wallet
        .add_label(funding_txid, "Payment for Machinery")
        .expect("Failed to add transaction label");

    let address_label = test_labelled_wallet
        .add_label(as_unchecked(dummy_address.clone()), "Employee address")
        .expect("Failed to add address label");

    let pubkey_label = test_labelled_wallet
        .add_label(dummy_pubkey, "My wallet's public key")
        .expect("Failed to add address label");

    let input_label = test_labelled_wallet
        .add_label(InputTarget(spent_outpoint), "My transaction's input")
        .expect("Failed to add address label");

    let output_label = test_labelled_wallet
        .add_label(OutputTarget(unspent_outpoint), "My transaction's Output")
        .expect("Failed to add address label");

    let xpub_label = test_labelled_wallet
        .add_label(dummy_xpub, "My wallet's extended public key")
        .expect("Failed to add address label");

    assert_eq!(test_labelled_wallet.labels.len(), 6);

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

    test_labelled_wallet
        .persist(&mut integration_mock_db)
        .expect("Failed to persist");

    assert_eq!(integration_mock_db.received_changesets.len(), 1);

    let mut buffer = Vec::new();

    test_labelled_wallet
        .export_labels(&mut buffer)
        .expect("Failed to export labels");

    assert!(
        !buffer.is_empty(),
        "The exported buffer should contain data"
    );

    let external_desc = "wpkh(0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798)";
    let internal_desc = "wpkh(03a0434d9e47f3c86235477c7b1ae6ae5d3442d49b1943c2b752a68e2a47e247c7)";

    let mut dest_wallet = Wallet::create(external_desc, internal_desc)
        .network(Network::Testnet)
        .create_wallet_no_persist()
        .expect("Failed to create destination wallet");

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

    assert_eq!(dest_labelled_wallet.labels.len(), 6);

    assert_eq!(
        dest_labelled_wallet.labels.get(&address_label.ref_()),
        Some(&address_label)
    );
}

#[test]
fn test_database_error_bubbles_up() {
    let mut failing_mock_db = FailingMockDb {};

    let (mut test_wallet, funding_txid) = funded_test_wallet();

    let mut test_changeset = LabelChangeset::new();

    let mut test_labelled_wallet = LabelledWallet {
        wallet: &mut test_wallet,
        labels: &mut test_changeset,
    };

    assert_eq!(test_labelled_wallet.labels.len(), 0);

    let transaction_label = test_labelled_wallet
        .add_label(funding_txid, "Payment for Machinery")
        .expect("Failed to add transaction label");

    assert_eq!(test_labelled_wallet.labels.len(), 1);

    assert!(matches!(
        transaction_label,
        Label::Transaction(TransactionRecord {
            ref_: _,
            label: Some(_),
            origin: _,
        })
    ));

    let result = test_labelled_wallet.persist(&mut failing_mock_db);

    assert!(matches!(result, Err(Error::Custom(_))));
}
