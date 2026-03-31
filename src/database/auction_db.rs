use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};

use poise::serenity_prelude::UserId;
use serde::{Deserialize, Serialize};

use crate::{
    Context, Res,
    database::{supported_locale::SupportedLocale, trade_db::MessageInfo},
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
}

/// A currently running auction. Once it expires and is resolved,
/// it gets moved into the trades database as `TradeKind::Auction`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningAuction {
    pub seller: UserId,

    pub item: Item,
    pub quantity: u16,

    pub currency_item: Item,
    pub min_price: u16,

    /// `UserId` -> their current bid amount
    pub bids: HashMap<UserId, u16>,

    pub start_time: SystemTime,
    pub end_time: SystemTime,
    pub duration: Duration,

    pub locale: SupportedLocale,

    pub english_message_id: MessageInfo,
    pub korean_message_id: MessageInfo,
}

impl RunningAuction {
    #[must_use]
    pub fn new(
        seller: UserId,
        item: Item,
        quantity: u16,
        currency_item: Item,
        min_price: u16,
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
        }
    }

    /// Returns the current highest bid, or `None` if no bids yet.
    #[must_use]
    pub fn highest_bid(&self) -> Option<(UserId, u16)> {
        self.bids
            .iter()
            .max_by_key(|(_, amount)| **amount)
            .map(|(&id, &amount)| (id, amount))
    }

    /// Returns true if the auction has expired.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.end_time.elapsed().is_ok_and(|e| e > Duration::ZERO)
    }

    /// Returns the minimum valid next bid — either `min_price` if no bids,
    /// or highest bid + 1.
    #[must_use]
    pub fn min_next_bid(&self) -> Option<u16> {
        self.highest_bid()
            .map_or(Some(self.min_price), |(_, amount)| amount.checked_add(1))
    }

    /// Returns true if `amount` is a valid new bid for `user`.
    #[must_use]
    pub fn is_valid_bid(&self, user: UserId, amount: u16) -> bool {
        let Some(min_next_bid) = self.min_next_bid() else {
            return false;
        };

        if amount < min_next_bid {
            return false;
        }

        // Can't lower own bid
        if let Some(&own_bid) = self.bids.get(&user)
            && amount <= own_bid
        {
            return false;
        }
        true
    }

    pub async fn delete_messages(self, ctx: Context<'_>) -> Res<()> {
        let channels = ctx.data().auctions_channel;

        let eng_id = self.english_message_id.id();
        let kor_id = self.korean_message_id.id();

        let eng_result =
            channels.english.delete_message(ctx.http(), eng_id?).await;
        let kor_result =
            channels.korean.delete_message(ctx.http(), kor_id?).await;

        eng_result?;
        kor_result?;
        Ok(())
    }
}
