use daybreak::{FileDatabase, deser::Yaml};
use poise::serenity_prelude::ChannelId;

use crate::{Res, database::trade_db::TradeData};

pub mod trade_db;

pub type TradingDatabase = FileDatabase<TradeData, Yaml>;

pub struct Data {
    pub trades: TradingDatabase,
    pub trade_posting_channel: ChannelId,
    pub bot_menu_channel: ChannelId,
}

impl Data {
    pub fn new(trade_posting_channel: &str, menu_channel: &str) -> Res<Self> {
        Ok(Self {
            trades: TradingDatabase::load_from_path_or_default(
                "trading_db.yml",
            )?,
            trade_posting_channel: ChannelId::new(
                trade_posting_channel.parse()?,
            ),
            bot_menu_channel: ChannelId::new(menu_channel.parse()?),
        })
    }
}
