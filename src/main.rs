use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fs::File;
use std::process;

// TODO: Organize code in files
// TODO: Check for valid `amount`s

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

#[derive(Debug, Deserialize, Default)]
struct Account {
    id: u16,
    available: f32,
    held: f32,
    total: f32,
    locked: bool,
}

impl Account {
    fn new(id: u16) -> Self {
        Account {
            id,
            ..Default::default()
        }
    }
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
        // Operations return errors in case handling preferred in the future
        let result = match r.tx_type {
            Deposit => self.deposit(r),
            Withdrawal => self.withdrawal(r),
            Dispute => self.dispute(r),
            Resolve => self.resolve(r),
            Chargeback => self.chargeback(r),
        };
    }

    fn fold_transaction(mut self, r: TransactionRecord) -> Self {
        self.parse_transaction(r);
        self
    }

    fn deposit(&mut self, r: TransactionRecord) -> Result<(), Box<dyn Error>> {
        // Assume empty deposits are okay for opening new acocunt
        let amount = r.amount.unwrap_or(0.0);
        // Get or init account
        let acnt = match self.accounts.get_mut(&r.client_id) {
            Some(acnt) => acnt,
            None => {
                let mut acnt = Account::new(r.client_id);
                self.accounts.insert(r.client_id, acnt);
                // Insert doesn't fail, we can unwrap safely
                self.accounts.get_mut(&r.client_id).unwrap()
            }
        };

        // update account
        acnt.total += amount;
        acnt.available += amount;

        // save tx
        self.save_tx(r)
    }

    fn save_tx(&mut self, r: TransactionRecord) -> Result<(), Box<dyn Error>> {
        match self.txs.insert(r.tx_id, r) {
            Some(_) => Err(From::from("transaction error: multiple tx's with same id")),
            None => Ok(()),
        }
    }

    fn withdrawal(&mut self, r: TransactionRecord) -> Result<(), Box<dyn Error>> {
        // Empty withdrawal are allowed, otherwise we'd use `ok_or`
        let amount = r.amount.unwrap_or(0.0);

        // Must have an account
        let acnt = self
            .accounts
            .get_mut(&r.client_id)
            .ok_or::<Box<dyn Error>>(From::from(
                "transaction error: withdrawal from non-existent client id",
            ))?;

        // Must have enough to withdraw
        if acnt.available < amount {
            return Err(From::from(
                "transaction error: withdrawl amount greater than available",
            ));
        }

        // update account
        acnt.available -= amount;
        acnt.total -= amount;

        self.save_tx(r)
    }

    fn dispute(&mut self, r: TransactionRecord) -> Result<(), Box<dyn Error>> {
        println!("{:#?}", r);
        Ok(())
    }

    fn resolve(&mut self, r: TransactionRecord) -> Result<(), Box<dyn Error>> {
        println!("{:#?}", r);
        Ok(())
    }

    fn chargeback(&mut self, r: TransactionRecord) -> Result<(), Box<dyn Error>> {
        println!("{:#?}", r);
        Ok(())
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
