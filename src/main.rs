use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fs::File;
use std::process;
use serde::Deserialize;

// TODO: Organize code in files
// TODO: Add all record types to transasctions.csv

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

fn run() -> Result<(), Box<dyn Error>> {
    let file_path = get_first_arg()?;
    let file = File::open(file_path)?;
    // let mut rdr = csv::Reader::from_reader(file);
    let mut rdr = csv::ReaderBuilder::new()
    .trim(csv::Trim::All).from_reader(file);
    for result in rdr.deserialize() {
        let record: TransactionRecord = result?;
        println!("{:#?}", record);
    }
    Ok(())
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
