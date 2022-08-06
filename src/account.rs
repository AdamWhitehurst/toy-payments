use serde::{ser::Serializer, Deserialize, Serialize};
use std::primitive::str;

#[derive(Debug, Deserialize, Default, Serialize)]
/// Represents a Client's account holding some amount of BTC,
/// tracking whether the account is frozen.
pub struct Account {
    #[serde(rename = "client")]
    pub id: u16,
    #[serde(rename = "available")]
    #[serde(serialize_with = "precision_four")]
    pub available: f32,
    #[serde(rename = "held")]
    #[serde(serialize_with = "precision_four")]
    pub held: f32,
    #[serde(rename = "total")]
    #[serde(serialize_with = "precision_four")]
    pub total: f32,
    #[serde(rename = "locked")]
    pub frozen: bool,
}

impl Account {
    pub fn new(id: u16) -> Self {
        Account {
            id,
            ..Default::default()
        }
    }
}

/// Serializes a given `f32` with a precision of four decimals
fn precision_four<S>(id: &f32, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let p_four_str = format!("{:.4}", id);
    s.serialize_str(&p_four_str.as_str())
}
