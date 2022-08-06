use crate::account::*;
use crate::transaction::*;

use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::Stdout;

/// Manages client `Account`s, tracks `TransactionRecord`s corresponding
/// to those `Account`s and handles disputes.
pub struct Ledger {
    /// Maps between client ID's and their corresponding accounts
    accounts: HashMap<u16, Account>,
    // Maps between transaction ID's and their corresponding record
    txs: HashMap<u32, TransactionRecord>,
    // Transactions that are currently under dispute
    disputed: HashMap<u32, ()>,
}

impl Ledger {
    /// Creates a new, empty `Ledger`
    fn new() -> Self {
        Ledger {
            accounts: HashMap::new(),
            txs: HashMap::new(),
            disputed: HashMap::new(),
        }
    }

    /// Creates a new, `Ledger` from a given csv file `Reader`
    pub fn from_reader(rdr: &mut csv::Reader<File>) -> Ledger {
        rdr.deserialize()
            // Only valid tx's
            .filter_map(|r| r.ok())
            // Parse each record into ledger
            .fold(Ledger::new(), |acc, r| acc.fold_transaction(r))
    }

    /// Writes the `Ledger`'s `Account`s to the given csv `Writer`
    pub fn write_accounts(&self, wtr: &mut csv::Writer<Stdout>) -> Result<(), Box<dyn Error>> {
        for (_, r) in self.accounts.iter() {
            wtr.serialize(r)?;
        }

        wtr.flush()?;

        Ok(())
    }

    /// Parses the given `TransactionRecord`, and returns `self`.
    /// for use with `fold`-type operations
    fn fold_transaction(mut self, r: TransactionRecord) -> Self {
        self.parse_transaction(r);
        self
    }

    /// Updates `Ledger` with the given `TransactionRecord`
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

    /// Handles the given `TransactionRecord` as a deposit transaction
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

    /// Handles the given `TransactionRecord` as a withdrawal transaction
    fn withdrawal(&mut self, r: TransactionRecord) -> Result<(), Box<dyn Error>> {
        // Avoid wasteful bookkeeping by disallowing empty deposits
        let amount = Ledger::expect_amount(&r)?;

        // Must have an account
        let acnt = self.expect_account(&r.client_id)?;

        Ledger::expect_not_frozen(&acnt)?;

        // Must have enough to withdraw
        if acnt.available < amount {
            return Err(From::from(format!("withdrawal amount greater than available, {:?}", acnt)));
        }

        // update account
        acnt.available -= amount;
        acnt.total -= amount;

        self.save_tx(r)
    }

    /// Handles the given `TransactionRecord` as a dispute transaction
    fn dispute(&mut self, r: TransactionRecord) -> Result<(), Box<dyn Error>> {
        let (d_amount, d_tx_id) = {
            // Must have a transaction
            let tx = self.try_get_tx(&r.tx_id)?;
            // Saved tx's should have an amount
            let amount = Ledger::expect_amount(&tx)?;
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

    /// Handles the given `TransactionRecord` as a resolution transaction
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

    /// Handles the given `TransactionRecord` as a chargeback transaction
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
        acnt.total -= d_amount;

        Ok(())
    }

    /// Saves the given `TransactionRecord` for later reference.
    /// Expected to only be used for deposits and withdrawals
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

    /// Gets the `Account` for the given `id` or throws if not found
    fn expect_account(&mut self, id: &u16) -> Result<&mut Account, Box<dyn Error>> {
        self.accounts
            .get_mut(id)
            .ok_or::<Box<dyn Error>>(From::from(format!(
                "cannot get account for non-existent client id: {}",
                id
            )))
    }

    /// Gets the `TransactionRecord` for the given `id` or throws if not found
    fn try_get_tx(&self, id: &u32) -> Result<&TransactionRecord, Box<dyn Error>> {
        self.txs.get(id).ok_or::<Box<dyn Error>>(From::from(format!(
            "cannot get record for non-existent tx id: {}",
            id
        )))
    }

    /// Gets the `amount` from the given `TransactionRecord` or throws if amount is `None`
    fn expect_amount(r: &TransactionRecord) -> Result<f32, Box<dyn Error>> {
        r.amount
            .ok_or::<Box<dyn Error>>(From::from(format!("expected amount for ID: {}", r.tx_id)))
    }

    /// Throws an error if the given `Account` is frozen/locked
    fn expect_not_frozen(acnt: &Account) -> Result<(), Box<dyn Error>> {
        match acnt.frozen {
            true => Err(From::from(format!("account frozen, ID: {}", acnt.id))),
            false => Ok(()),
        }
    }
}
