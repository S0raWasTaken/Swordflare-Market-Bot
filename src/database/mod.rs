use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use daybreak::{FileDatabase, deser::Yaml};
use poise::serenity_prelude::{ChannelId, RoleId, UserId};

use crate::{
    Res,
    database::{supported_locale::SupportedLocale, trade_db::TradeData},
};

pub mod supported_locale;
pub mod trade_db;

pub type TradingDatabase = FileDatabase<TradeData, Yaml>;
pub type LanguageDatabase =
    FileDatabase<HashMap<UserId, SupportedLocale>, Yaml>;
pub type Blacklist = FileDatabase<HashSet<UserId>, Yaml>;

#[derive(Clone, Copy)]
pub struct DoubleChannelId {
    english: ChannelId,
    korean: ChannelId,
}

impl DoubleChannelId {
    pub fn new(english_channel: &str, korean_channel: &str) -> Res<Self> {
        Ok(Self {
            english: ChannelId::new(english_channel.parse()?),
            korean: ChannelId::new(korean_channel.parse()?),
        })
    }

    /// Either grabs the korean channel or defaults
    /// to the english channel, don't overthink it.
    pub fn get_channel(&self, locale: SupportedLocale) -> ChannelId {
        match locale {
            SupportedLocale::ko_KR => self.korean,
            _ => self.english,
        }
    }
}

#[derive(Clone)]
pub struct Data {
    pub trades: Arc<TradingDatabase>,
    pub languages: Arc<LanguageDatabase>,
    pub blacklist: Arc<Blacklist>,
    pub trade_posting_channel: DoubleChannelId,
    #[expect(unused, reason = "Will be the last thing to be implemented")]
    pub bot_menu_channel: DoubleChannelId,
    pub admin_role: RoleId,
}

impl Data {
    pub fn new(
        english_posting_channel: &str,
        korean_posting_channel: &str,

        english_menu_channel: &str,
        korean_menu_channel: &str,

        admin_role_id: &str,
    ) -> Res<Self> {
        Ok(Self {
            trades: Arc::new(TradingDatabase::load_from_path_or_default(
                "trading_db.yml",
            )?),
            languages: Arc::new(LanguageDatabase::load_from_path_or_default(
                "languages.yml",
            )?),
            blacklist: Arc::new(Blacklist::load_from_path_or_default(
                "blacklist.yml",
            )?),
            trade_posting_channel: DoubleChannelId::new(
                english_posting_channel,
                korean_posting_channel,
            )?,
            bot_menu_channel: DoubleChannelId::new(
                english_menu_channel,
                korean_menu_channel,
            )?,
            admin_role: RoleId::new(admin_role_id.parse()?),
        })
    }
}
