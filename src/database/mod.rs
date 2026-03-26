use daybreak::{FileDatabase, deser::Yaml};

use crate::database::trade_db::TradeData;

pub mod trade_db;

pub type TradingDatabase = FileDatabase<TradeData, Yaml>;

pub struct Data {
    pub trades: TradingDatabase,
}

impl Data {
    pub fn new() -> Result<Self, daybreak::Error> {
        Ok(Self {
            trades: TradingDatabase::load_from_path_or_default(
                "trading_db.yml",
            )?,
        })
    }
}
