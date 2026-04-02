use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering::Relaxed},
    },
};

use daybreak::{FileDatabase, deser::Yaml};
use poise::serenity_prelude::{
    CacheHttp, ChannelId, CreateAllowedMentions, CreateMessage, RoleId, UserId,
};

use crate::{
    Error, Res,
    database::{
        auction_db::AuctionData,
        supported_locale::SupportedLocale,
        trade_db::{Trade, TradeData, TradeStatus},
    },
    get_vars,
};

pub mod auction_db;
pub mod supported_locale;
pub mod trade_db;

pub type TradingDatabase = FileDatabase<TradeData, Yaml>;
pub type LanguageDatabase =
    FileDatabase<HashMap<UserId, SupportedLocale>, Yaml>;
pub type Blacklist = FileDatabase<HashSet<UserId>, Yaml>;
pub type RunningAuctions = FileDatabase<AuctionData, Yaml>;

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

    /// 0: English Channel, 1: Korean Channel
    pub fn get_both(&self) -> (ChannelId, ChannelId) {
        (self.english, self.korean)
    }
}

#[derive(Clone)]
pub struct Data {
    pub trades: Arc<TradingDatabase>,
    pub running_auctions: Arc<RunningAuctions>,
    pub languages: Arc<LanguageDatabase>,
    pub blacklist: Arc<Blacklist>,
    pub trades_channel: DoubleChannelId,
    pub auctions_channel: DoubleChannelId,
    pub logs_channel: ChannelId,
    pub reports_channel: ChannelId,
    pub admin_role: RoleId,
    paused: Arc<AtomicBool>,
}

impl Data {
    pub fn new() -> Res<(Self, String)> {
        let (
            token,
            english_posting_channel_id,
            korean_posting_channel_id,
            english_auctions_channel_id,
            korean_auctions_channel_id,
            logs_channel_id,
            reports_channel_id,
            admin_role_id,
        ) = get_vars!(
            "DISCORD_TOKEN",
            "ENGLISH_POSTING_CHANNEL_ID",
            "KOREAN_POSTING_CHANNEL_ID",
            "ENGLISH_AUCTIONS_CHANNEL_ID",
            "KOREAN_AUCTIONS_CHANNEL_ID",
            "LOGS_CHANNEL_ID",
            "REPORTS_CHANNEL_ID",
            "ADMIN_ROLE_ID"
        );

        Ok((
            Self {
                trades: Arc::new(TradingDatabase::load_from_path_or_default(
                    "trading_db.yml",
                )?),
                running_auctions: Arc::new(
                    RunningAuctions::load_from_path_or_default(
                        "running_auctions.yml",
                    )?,
                ),
                languages: Arc::new(
                    LanguageDatabase::load_from_path_or_default(
                        "languages.yml",
                    )?,
                ),
                blacklist: Arc::new(Blacklist::load_from_path_or_default(
                    "blacklist.yml",
                )?),
                trades_channel: DoubleChannelId::new(
                    &english_posting_channel_id,
                    &korean_posting_channel_id,
                )?,
                auctions_channel: DoubleChannelId::new(
                    &english_auctions_channel_id,
                    &korean_auctions_channel_id,
                )?,
                logs_channel: ChannelId::new(logs_channel_id.parse()?),
                reports_channel: ChannelId::new(reports_channel_id.parse()?),
                admin_role: RoleId::new(admin_role_id.parse()?),
                paused: Arc::new(AtomicBool::new(false)),
            },
            token,
        ))
    }

    pub async fn log(
        &self,
        cache_http: impl CacheHttp,
        message: &str,
    ) -> Res<()> {
        let log_channel = self.logs_channel;
        log_channel
            .send_message(
                cache_http,
                CreateMessage::default().content(message).allowed_mentions(
                    CreateAllowedMentions::new()
                        .empty_roles()
                        .empty_users()
                        .everyone(false),
                ),
            )
            .await?;
        Ok(())
    }

    pub fn find_duplicate_trade(&self, trade: &Trade) -> Res<Option<Trade>> {
        Ok(self.trades.read(|db| {
            db.iter().find_map(|t| {
                if matches!(t.1.status(), TradeStatus::Running) && t.1 == trade
                {
                    Some(t.1.clone())
                } else {
                    None
                }
            })
        })?)
    }

    pub fn new_report(
        &self,
        reporter: UserId,
        report: String,
        trade_id: u64,
        locale: &str,
    ) -> Res<(bool, String)> {
        self.trades.write(|db| {
            let trade = db
                .get_mut(trade_id)
                .ok_or(t!("error.trade_not_found", locale = locale))?;

            // This doesn't try reading Data::trades, so it should be dealock safe.
            let link = trade.message_link(self, SupportedLocale::en_US)?;

            Ok::<(bool, String), Error>((
                trade.add_report(reporter, report),
                link,
            ))
        })?
    }

    pub fn pause(&self) -> bool {
        !self.paused.swap(true, Relaxed)
    }

    pub fn resume(&self) -> bool {
        self.paused.swap(false, Relaxed)
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Relaxed)
    }
}
