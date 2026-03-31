use std::{
    collections::{HashMap, HashSet},
    time::{Duration, SystemTime},
};

use poise::serenity_prelude::{ChannelId, Context, MessageId, UserId};
use serde::{Deserialize, Serialize};

use crate::{
    Res,
    database::{
        Data, auction_db::RunningAuction, supported_locale::SupportedLocale,
    },
    items::Item,
    magic_numbers::{MODERATION_HOLD_PERIOD, TRADE_EXPIRATION_TIME},
    print_err,
};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TradeData {
    inner: HashMap<u64, Trade>,
    next_id: u64,
}

impl TradeData {
    #[inline]
    pub fn insert(&mut self, trade: Trade) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.inner.insert(id, trade);
        id
    }

    #[inline]
    pub fn remove(&mut self, id: u64) -> Option<Trade> {
        self.inner.remove(&id)
    }

    #[inline]
    #[must_use]
    pub fn get(&self, id: u64) -> Option<&Trade> {
        self.inner.get(&id)
    }

    #[inline]
    #[must_use]
    pub fn get_mut(&mut self, id: u64) -> Option<&mut Trade> {
        self.inner.get_mut(&id)
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (u64, &Trade)> {
        self.inner.iter().map(|(&id, trade)| (id, trade))
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub enum TradeKind {
    Normal,
    Auction,
}

#[derive(Clone, Copy, Debug)]
pub enum TradeStatus {
    /// Post message is up and running.
    Running,
    /// Expired, Soldout, waiting for possible moderation and deletion
    /// or a refresh / edit.
    Timeout,
    /// Post moderated or expired with zero buyers.
    /// Queue message for deletion.
    Invalid,
    /// Queue message for deletion, post had buyers and now it's
    /// kept in the database for telemetry purposes.
    Historical,
}

impl From<&Trade> for TradeStatus {
    fn from(value: &Trade) -> Self {
        let inactive = value.is_inactive();
        let had_zero_buyers = value.buyers.is_empty();

        if value.moderated {
            return Self::Invalid;
        }

        if !inactive {
            return Self::Running;
        }

        if had_zero_buyers {
            return Self::Invalid;
        }

        // by now, we know it's not active, but it had at least one buyer,
        // so it's either waiting for moderation or it's allowed to rest in peace,
        // buried in the database.

        if value
            .last_updated
            .elapsed()
            .is_ok_and(|e| e > MODERATION_HOLD_PERIOD)
        {
            Self::Historical
        } else {
            Self::Timeout
        }
    }
}

#[derive(Default, Serialize, Deserialize, Debug, Clone, Copy)]
pub struct MessageInfo {
    id: Option<MessageId>,
    inserted: bool,
    pub deleted: bool,
}

impl MessageInfo {
    pub fn insert(&mut self, id: MessageId) {
        self.id = Some(id);
        self.inserted = true;
    }

    const ERROR_MSG: &str = "MessageID was not inserted";

    /// NOP if the message was already deleted
    pub async fn delete(
        &mut self,
        ctx: &Context,
        channel: ChannelId,
    ) -> Res<()> {
        if self.deleted {
            return Ok(());
        }

        if self.inserted {
            channel
                .delete_message(&ctx.http, self.id.unwrap())
                .await
                .inspect_err(print_err)?;
            self.deleted = true;
            return Ok(());
        }
        Err(Self::ERROR_MSG.into())
    }

    pub fn id(&self) -> Res<MessageId> {
        self.id.ok_or(Self::ERROR_MSG.into())
    }

    #[inline]
    pub fn is_eq(&self, id: MessageId) -> bool {
        self.id == Some(id)
    }
}

/// Defines a single trade
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    // Basic stuff
    pub seller: UserId,
    pub item: Item,    // Seller will give this item
    pub quantity: u16, // Seller will give this amount
    pub wants: Item,
    pub wanted_amount: u16, // Seller wants this amount
    pub stock: u16,         // How many times this trade can be done

    // Technical stuff
    pub kind: TradeKind,
    pub locale: SupportedLocale,

    last_updated: SystemTime,
    created_at: SystemTime,

    pub duration: Duration,

    pub buyers: HashSet<UserId>,

    pub english_message_id: MessageInfo,
    pub korean_message_id: MessageInfo,

    pub moderated: bool,
}

impl From<(RunningAuction, Option<UserId>)> for Trade {
    fn from((auction, winner): (RunningAuction, Option<UserId>)) -> Self {
        let highest_bid =
            auction.highest_bid().unwrap_or((UserId::default(), 0));

        Self {
            seller: auction.seller,
            item: auction.item,
            quantity: auction.quantity,
            wants: auction.currency_item,
            wanted_amount: highest_bid.1,
            stock: 1,
            kind: TradeKind::Auction,
            locale: auction.locale,
            last_updated: auction.start_time,
            created_at: auction.start_time,
            duration: auction.duration,
            buyers: winner.into_iter().collect(),
            english_message_id: auction.english_message_id,
            korean_message_id: auction.korean_message_id,
            moderated: false,
        }
    }
}

impl PartialEq for Trade {
    fn eq(&self, other: &Self) -> bool {
        self.seller == other.seller
            && self.item == other.item
            && self.quantity == other.quantity
            && self.wants == other.wants
            && self.wanted_amount == other.wanted_amount
    }
}

impl Trade {
    #[must_use]
    #[expect(clippy::too_many_arguments)]
    pub fn new(
        user: UserId,
        trade_item: Item,
        trade_quantity: u16,
        wants: Item,
        amount: u16,
        stock: u16,
        trade_kind: TradeKind,
        locale: SupportedLocale,
    ) -> Self {
        let time_of_creation = SystemTime::now();
        Self {
            seller: user,
            item: trade_item,
            quantity: trade_quantity,
            wants,
            wanted_amount: amount,
            stock,
            last_updated: time_of_creation,
            created_at: time_of_creation,
            duration: TRADE_EXPIRATION_TIME,
            buyers: HashSet::new(),
            english_message_id: MessageInfo::default(),
            korean_message_id: MessageInfo::default(),
            moderated: false,
            kind: trade_kind,
            locale,
        }
    }

    #[inline]
    pub fn is_inactive(&self) -> bool {
        self.last_updated
            .elapsed()
            .is_ok_and(|elapsed| elapsed > self.duration) // Treat clock regression as not expired
            || self.is_sold_out()
            || self.moderated
    }

    #[inline]
    #[must_use]
    pub fn is_sold_out(&self) -> bool {
        self.stock == 0
    }

    // Don't even risk callers (me, myself and I) from editing this field, lol
    #[inline]
    #[must_use]
    pub fn created_at(&self) -> SystemTime {
        self.created_at
    }

    #[inline]
    #[must_use]
    pub fn status(&self) -> TradeStatus {
        TradeStatus::from(self)
    }

    pub async fn delete_messages(
        &mut self,
        ctx: &Context,
        data: &Data,
    ) -> Res<()> {
        let (english_channel, korean_channel) = match self.kind {
            TradeKind::Normal => data.trades_channel.get_both(),
            TradeKind::Auction => data.auctions_channel.get_both(),
        };

        let res_1 = self
            .korean_message_id
            .delete(ctx, korean_channel)
            .await
            .inspect_err(print_err);
        self.english_message_id
            .delete(ctx, english_channel)
            .await
            .inspect_err(print_err)?;

        res_1?;
        Ok(())
    }
}
