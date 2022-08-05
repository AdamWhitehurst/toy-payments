use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fs::File;
use std::process;

// TODO: Organize code in files

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Deserialize)]
struct TransactionRecord {
    #[serde(rename = "type")]
    tx_type: TransactionType,
    #[serde(rename = "client")]
    client_id: u16,
    #[serde(rename = "tx")]
    tx_id: u32,
    #[serde(rename = "amount")]
    #[serde(deserialize_with = "csv::invalid_option")]
    amount: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct Account {
    id: u16,
    available: f32,
    held: f32,
    total: f32,
    locked: bool,
}

// MAYBE: Generic over type of account?
struct Ledger {
    /// Maps between client ID's and their corresponding accounts
    accounts: HashMap<u16, Account>,
}

impl Ledger {
    fn new() -> Self {
        Ledger {
            accounts: HashMap::new(),
        }
    }
    fn fold_transaction(mut self, r: TransactionRecord) -> Self {
        println!("{:#?}", r);
        self
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let file_path = get_first_arg()?;
    let file = File::open(file_path)?;
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(file);
    let ldgr = rdr
        .deserialize()
        .filter_map(into_tx)
        .fold(Ledger::new(), |acc, r| acc.fold_transaction(r));
    Ok(())
}

fn into_tx(r: Result<TransactionRecord, csv::Error>) -> Option<TransactionRecord> {
    // Error handling could happen here
    r.ok()
}

/// Returns the first positional argument sent to this process. If there are no
/// positional arguments, then this returns an error.
fn get_first_arg() -> Result<OsString, Box<dyn Error>> {
    match env::args_os().nth(1) {
        None => Err(From::from("expected 1 argument, but got none")),
        Some(file_path) => Ok(file_path),
    }
}

fn main() {
    if let Err(err) = run() {
        println!("{}", err);
        process::exit(1);
    }
}
