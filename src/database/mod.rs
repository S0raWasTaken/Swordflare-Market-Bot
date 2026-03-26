use daybreak::{FileDatabase, deser::Yaml};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Dummy;

pub type TradingDatabase = FileDatabase<Dummy, Yaml>;
