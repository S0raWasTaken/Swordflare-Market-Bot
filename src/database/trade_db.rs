use std::{
    collections::{HashMap, HashSet},
    time::{Duration, SystemTime},
};

use poise::serenity_prelude::{MessageId, UserId};
use serde::{Deserialize, Serialize};

use crate::items::Item;

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
    created_at: SystemTime,

    pub buyers: HashSet<UserId>,

    pub message_id: Option<MessageId>,
}

/// Duration until a trade expires (2 days)
pub const EXPIRATION_TIME: Duration = Duration::from_hours(2 * 24); // TODO: Discuss about this number

impl Trade {
    #[must_use]
    pub fn new(
        user: UserId,
        trade_item: Item,
        trade_quantity: u16,
        wants: Item,
        amount: u16,
        stock: u16,
    ) -> Self {
        Self {
            seller: user,
            item: trade_item,
            quantity: trade_quantity,
            wants,
            wanted_amount: amount,
            stock,
            created_at: SystemTime::now(),
            buyers: HashSet::new(),
            message_id: None,
        }
    }

    /// # Panics
    ///
    /// Panics if the system clock has gone backwards since the trade was created.
    #[inline]
    #[expect(dead_code, reason = "Future implementation")]
    pub fn is_inactive(&self) -> bool {
        self.created_at
            .elapsed()
            .is_ok_and(|elapsed| elapsed > EXPIRATION_TIME) // Treat clock regression as not expired
            || self.is_sold_out()
    }

    #[inline]
    #[must_use]
    pub fn is_sold_out(&self) -> bool {
        self.stock == 0
    }
}
