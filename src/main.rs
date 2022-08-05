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
    // Maps between transaction ID's and their corresponding record
    txs: HashMap<u32, TransactionRecord>,
}

impl Ledger {
    fn new() -> Self {
        Ledger {
            accounts: HashMap::new(),
            txs: HashMap::new(),
        }
    }
    fn parse_transaction(&mut self, r: TransactionRecord) {
        use TransactionType::*;
        match r.tx_type {
            Deposit => self.deposit(r),
            Withdrawal => self.withdrawal(r),
            Dispute => self.dispute(r),
            Resolve => self.resolve(r),
            Chargeback => self.chargeback(r),
        }
    }

    fn fold_transaction(mut self, r: TransactionRecord) -> Self {
        self.parse_transaction(r);
        self
    }

    fn deposit(&mut self, r: TransactionRecord) {
        println!("{:#?}", r);
    }

    fn withdrawal(&mut self, r: TransactionRecord) {
        println!("{:#?}", r);
    }

    fn dispute(&mut self, r: TransactionRecord) {
        println!("{:#?}", r);
    }

    fn resolve(&mut self, r: TransactionRecord) {
        println!("{:#?}", r);
    }

    fn chargeback(&mut self, r: TransactionRecord) {
        println!("{:#?}", r);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    // Try to read file from arg name
    let file_path = get_first_arg()?;
    let file = File::open(file_path)?;
    // Make a CSV reader
    let mut rdr = csv::ReaderBuilder::new()
        // Support spaces between fields
        .trim(csv::Trim::All)
        .from_reader(file);
    // Compose a Ledger from csv lines
    let ldgr = rdr
        .deserialize()
        // Only valid tx's
        .filter_map(into_tx)
        // Parse each record into ledger
        .fold(Ledger::new(), |acc, r| acc.fold_transaction(r));

    // TODO: Serialize Ledger

    Ok(())
}

/// Converts Results into Options
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
