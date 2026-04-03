use std::{
    collections::{HashMap, hash_map::IterMut},
    time::{Duration, SystemTime},
};

use poise::serenity_prelude::{self, UserId};
use serde::{Deserialize, Serialize};

use crate::{
    ACTIVE_GUILD_ID, Res,
    database::{
        Data, supported_locale::SupportedLocale, trade_db::MessageInfo,
    },
    items::Item,
};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AuctionData {
    inner: HashMap<u64, RunningAuction>,
    next_id: u64,
}

impl AuctionData {
    #[inline]
    pub fn insert(&mut self, auction: RunningAuction) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.inner.insert(id, auction);
        id
    }

    #[inline]
    pub fn remove(&mut self, id: u64) -> Option<RunningAuction> {
        self.inner.remove(&id)
    }

    #[inline]
    #[must_use]
    pub fn get(&self, id: u64) -> Option<&RunningAuction> {
        self.inner.get(&id)
    }

    #[inline]
    #[must_use]
    pub fn get_mut(&mut self, id: u64) -> Option<&mut RunningAuction> {
        self.inner.get_mut(&id)
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (u64, &RunningAuction)> {
        self.inner.iter().map(|(&id, a)| (id, a))
    }

    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, u64, RunningAuction> {
        self.inner.iter_mut()
    }
}

/// A currently running auction. Once it expires and is resolved,
/// it gets moved into the trades database as `TradeKind::Auction`.
#[expect(clippy::unsafe_derive_deserialize, reason = "tokio::join!")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningAuction {
    pub seller: UserId,

    pub item: Item,
    pub quantity: u64,

    pub currency_item: Item,
    pub min_price: u64,

    /// `UserId` -> their current bid amount
    pub bids: HashMap<UserId, u64>,

    pub start_time: SystemTime,
    pub end_time: SystemTime,
    pub duration: Duration,

    pub locale: SupportedLocale,

    pub english_message_id: MessageInfo,
    pub korean_message_id: MessageInfo,

    pub is_being_handled: bool,
}

impl RunningAuction {
    #[must_use]
    pub fn new(
        seller: UserId,
        item: Item,
        quantity: u64,
        currency_item: Item,
        min_price: u64,
        duration: Duration,
        locale: SupportedLocale,
    ) -> Self {
        let start_time = SystemTime::now();
        Self {
            seller,
            item,
            quantity,
            currency_item,
            min_price,
            bids: HashMap::new(),
            start_time,
            end_time: start_time + duration,
            duration,
            locale,
            english_message_id: MessageInfo::default(),
            korean_message_id: MessageInfo::default(),
            is_being_handled: false,
        }
    }

    /// Returns the current highest bid, or `None` if no bids yet.
    #[must_use]
    pub fn highest_bid(&self) -> Option<(UserId, u64)> {
        self.bids
            .iter()
            .max_by_key(|(_, amount)| **amount)
            .map(|(&id, &amount)| (id, amount))
    }

    pub fn sorted_bid_list(&self, locale: &str) -> String {
        let mut bids: Vec<(UserId, u64)> =
            self.bids.iter().map(|(&id, &amt)| (id, amt)).collect();
        bids.sort_by_key(|b| std::cmp::Reverse(b.1));

        let currency = self.currency_item.name.display(locale);
        bids.iter()
            .map(|(id, amt)| format!("**{amt} {currency}** — <@{id}>"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Returns true if the auction has expired.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.end_time.elapsed().is_ok()
    }

    /// Returns the minimum valid next bid for a user — at least `min_price`,
    /// greater than the user's current bid, and not colliding with any existing bid.
    #[must_use]
    pub fn min_next_bid(&self, user: UserId) -> u64 {
        let current = self.bids.get(&user).copied().unwrap_or(0);
        let mut candidate = self.min_price.max(current + 1);
        while self.bids.values().any(|&b| b == candidate) {
            candidate += 1;
        }
        candidate
    }

    #[inline]
    #[must_use]
    pub fn is_valid_bid(&self, user: UserId, amount: u64) -> bool {
        amount >= self.min_next_bid(user)
            && !self.bids.iter().any(|(&id, &bid)| id != user && bid == amount)
    }

    pub fn message_link(
        &self,
        locale: SupportedLocale,
        data: &Data,
    ) -> Res<String> {
        let guild_id = *ACTIVE_GUILD_ID;
        let channel_id = data.auctions_channel.get_channel(locale);
        let message_id = match locale {
            SupportedLocale::ko_KR => self.korean_message_id.id(),
            _ => self.english_message_id.id(),
        }?;

        Ok(format!(
            "https://discord.com/channels/{guild_id}/{channel_id}/{message_id}"
        ))
    }

    pub fn display_log(&self, data: &Data) -> Res<String> {
        let seller_id = self.seller;

        let auction_display = self.display_simple("en-US");
        let link = self.message_link(SupportedLocale::en_US, data)?;

        Ok(format!(
            "<@{seller_id}> started an auction: \
            {auction_display}.\n\
            {link}"
        ))
    }

    pub fn display_simple(&self, locale: &str) -> String {
        format!(
            "{} x{} for {} x{} minimum",
            self.item.name.display(locale),
            self.quantity,
            self.currency_item.name.display(locale),
            self.min_price
        )
    }

    pub async fn delete_messages(
        self,
        ctx: &serenity_prelude::Context,
        data: &Data,
    ) -> Res<()> {
        let channels = data.auctions_channel;

        let eng_id = self.english_message_id.id()?;
        let kor_id = self.korean_message_id.id()?;

        let (eng_result, kor_result) = tokio::join! {
                    channels.english.delete_message(ctx, eng_id),
                    channels.korean.delete_message(ctx, kor_id)
        };
        eng_result?;
        kor_result?;
        Ok(())
    }
}
