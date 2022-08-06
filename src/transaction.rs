use serde::Deserialize;

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Deserialize)]
/// Represents a csv record's transaction on a client's account
pub struct TransactionRecord {
    #[serde(rename = "type")]
    /// What operation to perform on the client's account
    pub tx_type: TransactionType,
    #[serde(rename = "client")]
    /// ID of client's account
    pub client_id: u16,
    #[serde(rename = "tx")]
    /// ID of transaction
    pub tx_id: u32,
    #[serde(rename = "amount")]
    #[serde(deserialize_with = "csv::invalid_option")]
    /// Value to withdraw/deposit into account
    pub amount: Option<f32>,
}
