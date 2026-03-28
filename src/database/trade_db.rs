use std::{
    collections::{HashMap, HashSet},
    time::SystemTime,
};

use poise::serenity_prelude::{Context, MessageId, UserId};
use serde::{Deserialize, Serialize};

use crate::{
    database::Data,
    items::Item,
    magic_numbers::{MODERATION_HOLD_PERIOD, TRADE_EXPIRATION_TIME},
    print_err,
};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TradeData {
    inner: HashMap<u64, Trade>,
    next_id: u64,
}

#[expect(dead_code)]
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

    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub enum TradeKind {
    Normal,
    Auction,
}

#[derive(Clone, Copy)]
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
    last_updated: SystemTime,
    created_at: SystemTime,

    pub buyers: HashSet<UserId>,

    pub message_id: Option<MessageId>,
    pub message_deleted: bool,

    pub moderated: bool,
}

impl Trade {
    #[must_use]
    pub fn new(
        user: UserId,
        trade_item: Item,
        trade_quantity: u16,
        wants: Item,
        amount: u16,
        stock: u16,
        trade_kind: TradeKind,
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
            buyers: HashSet::new(),
            message_id: None,
            message_deleted: false,
            moderated: false,
            kind: trade_kind,
        }
    }

    /// # Panics
    ///
    /// Panics if the system clock has gone backwards since the trade was created.
    #[inline]
    pub fn is_inactive(&self) -> bool {
        self.last_updated
            .elapsed()
            .is_ok_and(|elapsed| elapsed > TRADE_EXPIRATION_TIME) // Treat clock regression as not expired
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

    pub async fn delete_message(&mut self, ctx: &Context, data: &Data) -> bool {
        if !self.message_deleted
            && let Some(message_id) = self.message_id
        {
            let deleted = data
                .trade_posting_channel
                .delete_message(&ctx.http, message_id)
                .await
                .inspect_err(print_err)
                .is_ok();
            self.message_deleted = deleted;
            deleted
        } else {
            false
        }
    }
}
