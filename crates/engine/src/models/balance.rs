use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssetBalance {
    pub available: Decimal,
    pub locked: Decimal,
}

impl AssetBalance {
    pub fn new(available: Decimal, locked: Decimal) -> Self {
        Self { available, locked }
    }
}

pub type UserBalance = HashMap<String, AssetBalance>;
