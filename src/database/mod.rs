use std::sync::Arc;

use daybreak::{FileDatabase, deser::Yaml};
use poise::serenity_prelude::ChannelId;

use crate::{Res, database::trade_db::TradeData};

pub mod trade_db;

pub type TradingDatabase = FileDatabase<TradeData, Yaml>;

#[derive(Clone)]
pub struct Data {
    pub trades: Arc<TradingDatabase>,
    pub trade_posting_channel: ChannelId,
    #[expect(unused, reason = "Will be the last thing to be implemented")]
    pub bot_menu_channel: ChannelId,
}

impl Data {
    pub fn new(
        trade_posting_channel: &str,
        bot_menu_channel: &str,
    ) -> Res<Self> {
        Ok(Self {
            trades: Arc::new(TradingDatabase::load_from_path_or_default(
                "trading_db.yml",
            )?),
            trade_posting_channel: ChannelId::new(
                trade_posting_channel.parse()?,
            ),
            bot_menu_channel: ChannelId::new(bot_menu_channel.parse()?),
        })
    }
}
