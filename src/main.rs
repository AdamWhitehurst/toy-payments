use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env::args_os;
use std::error::Error;
use std::ffi::OsString;
use std::fs::File;
use std::io::{stdout, Stdout};
use std::process;

// TODO: Organize code in files
// TODO: Check for valid `amount`s

#[derive(Copy, Clone, Debug, Deserialize)]
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

#[derive(Debug, Deserialize, Default, Serialize)]
struct Account {
    #[serde(rename = "client")]
    id: u16,
    #[serde(rename = "available")]
    available: f32,
    #[serde(rename = "held")]
    held: f32,
    #[serde(rename = "total")]
    total: f32,
    #[serde(rename = "locked")]
    frozen: bool,
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
    // Transactions that are currently under dispute
    disputed: HashMap<u32, ()>,
}

impl Ledger {
    fn new() -> Self {
        Ledger {
            accounts: HashMap::new(),
            txs: HashMap::new(),
            disputed: HashMap::new(),
        }
    }

    fn from_reader(rdr: &mut csv::Reader<File>) -> Ledger {
        rdr.deserialize()
            // Only valid tx's
            .filter_map(into_tx)
            // Parse each record into ledger
            .fold(Ledger::new(), |acc, r| acc.fold_transaction(r))
    }

    fn write_accounts(&self, wtr: &mut csv::Writer<Stdout>) -> Result<(), Box<dyn Error>> {
        for (_, r) in self.accounts.iter() {
            wtr.serialize(r)?;
        }

        wtr.flush()?;

        Ok(())
    }

    fn fold_transaction(mut self, r: TransactionRecord) -> Self {
        self.parse_transaction(r);
        self
    }

    fn parse_transaction(&mut self, r: TransactionRecord) {
        use TransactionType::*;
        // Operations return errors in case error handling needed in the future
        // For now, they are ignored.
        let _ = match r.tx_type {
            Deposit => self.deposit(r),
            Withdrawal => self.withdrawal(r),
            Dispute => self.dispute(r),
            Resolve => self.resolve(r),
            Chargeback => self.chargeback(r),
        };
    }

    fn deposit(&mut self, r: TransactionRecord) -> Result<(), Box<dyn Error>> {
        // Avoid wasteful bookkeeping by disallowing empty deposits
        let amount = Ledger::expect_amount(&r)?;

        // Get or init account
        let acnt = match self.accounts.get_mut(&r.client_id) {
            Some(acnt) => acnt,
            None => {
                let acnt = Account::new(r.client_id);
                self.accounts.insert(r.client_id, acnt);
                // Insert doesn't fail, we can unwrap safely
                self.accounts.get_mut(&r.client_id).unwrap()
            }
        };

        Ledger::expect_not_frozen(&acnt)?;

        // update account
        acnt.total += amount;
        acnt.available += amount;

        // save tx
        self.save_tx(r)
    }

    fn save_tx(&mut self, r: TransactionRecord) -> Result<(), Box<dyn Error>> {
        match self.txs.insert(r.tx_id, r) {
            Some(first_tx) => {
                let tx_id = first_tx.tx_id;
                // Keep first one
                self.txs.insert(tx_id, first_tx);
                // Bail
                Err(From::from(format!(
                    "saved multiple tx's with same id: {}",
                    tx_id
                )))
            }
            None => Ok(()),
        }
    }

    fn withdrawal(&mut self, r: TransactionRecord) -> Result<(), Box<dyn Error>> {
        // Avoid wasteful bookkeeping by disallowing empty deposits
        let amount = Ledger::expect_amount(&r)?;

        // Must have an account
        let acnt = self.expect_account(&r.client_id)?;

        Ledger::expect_not_frozen(&acnt)?;

        // Must have enough to withdraw
        if acnt.available < amount {
            return Err(From::from("withdrawal amount greater than available"));
        }

        // update account
        acnt.available -= amount;
        acnt.total -= amount;

        self.save_tx(r)
    }

    fn dispute(&mut self, r: TransactionRecord) -> Result<(), Box<dyn Error>> {
        let (d_amount, d_tx_id) = {
            // Must have a transaction
            let tx = self.try_get_tx(&r.tx_id)?;
            // Saved tx's should have an amount
            let amount = Ledger::expect_amount(&r)?;
            (amount, tx.tx_id)
        };

        {
            self.disputed.insert(d_tx_id, ());
        }

        let acnt = self.expect_account(&r.client_id)?;

        acnt.available -= d_amount;
        acnt.held += d_amount;

        Ok(())
    }

    fn resolve(&mut self, r: TransactionRecord) -> Result<(), Box<dyn Error>> {
        let (d_amount, d_tx_id) = {
            let tx = self.try_get_tx(&r.tx_id)?;
            (tx.amount.unwrap_or(0.0), tx.tx_id)
        };

        self.disputed
            .get(&d_tx_id)
            .ok_or::<Box<dyn Error>>(From::from(format!("cannot resolve undisputed tx")))?;

        {
            self.disputed.remove(&d_tx_id);
        }

        let acnt = self.expect_account(&r.client_id)?;

        acnt.held -= d_amount;
        acnt.available += d_amount;

        Ok(())
    }

    fn chargeback(&mut self, r: TransactionRecord) -> Result<(), Box<dyn Error>> {
        let d_amount = {
            let tx = self.try_get_tx(&r.tx_id)?;
            // Allow for empty amounts because maybe client doesn't care it was worthless?
            // otherwise we could use `ok_or`
            tx.amount.unwrap_or(0.0)
        };

        let acnt = self.expect_account(&r.client_id)?;

        acnt.frozen = true;
        acnt.held -= d_amount;
        acnt.available -= d_amount;

        Ok(())
    }

    fn expect_account(&mut self, id: &u16) -> Result<&mut Account, Box<dyn Error>> {
        self.accounts
            .get_mut(id)
            .ok_or::<Box<dyn Error>>(From::from(format!(
                "cannot get account for non-existent client id: {}",
                id
            )))
    }

    fn try_get_tx(&self, id: &u32) -> Result<&TransactionRecord, Box<dyn Error>> {
        self.txs.get(id).ok_or::<Box<dyn Error>>(From::from(format!(
            "cannot get record for non-existent tx id: {}",
            id
        )))
    }

    fn expect_amount(r: &TransactionRecord) -> Result<f32, Box<dyn Error>> {
        r.amount
            .ok_or::<Box<dyn Error>>(From::from(format!("expected amount for ID: {}", r.tx_id)))
    }

    fn expect_not_frozen(acnt: &Account) -> Result<(), Box<dyn Error>> {
        match acnt.frozen {
            true => Err(From::from(format!("account frozen, ID: {}", acnt.id))),
            false => Ok(()),
        }
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
    let ldgr = Ledger::from_reader(&mut rdr);
    // Make a writer for writing Account records into
    let mut wtr = csv::Writer::from_writer(stdout());
    // Write accounts to
    ldgr.write_accounts(&mut wtr)?;

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
    match args_os().nth(1) {
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
