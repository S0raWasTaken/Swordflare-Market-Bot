use std::{
    collections::{HashMap, HashSet},
    time::{Duration, SystemTime, SystemTimeError},
};

use poise::serenity_prelude::UserId;
use serde::{Deserialize, Serialize};

use crate::items::Item;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TradeData {
    inner: HashMap<u64, Trade>,
    next_id: u64,
}

#[allow(dead_code)]
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
    seller: UserId,
    trades: Item, // Seller will give this item
    wants: Item,
    amount: u16, // Seller wants this amount
    stock: u16,  // Trades in stock

    // Technical stuff
    created_at: SystemTime,
    expires_in: Duration,

    buyers: HashSet<UserId>,
}

/// Hours till trade is expired
const EXPIRATION_TIME: u64 = 2 * 24; // TODO: Discuss about this number

impl Trade {
    #[must_use]
    pub fn new(
        user: UserId,
        trades: Item,
        wants: Item,
        amount: u16,
        stock: u16,
    ) -> Self {
        Self {
            seller: user,
            trades,
            wants,
            amount,
            stock,
            created_at: SystemTime::now(),
            expires_in: Duration::from_hours(EXPIRATION_TIME),
            buyers: HashSet::new(),
        }
    }

    #[inline]
    pub fn is_expired(&self) -> Result<bool, SystemTimeError> {
        self.created_at.elapsed().map(|created| created > self.expires_in)
    }

    #[inline]
    #[must_use]
    pub fn is_sold_out(&self) -> bool {
        self.stock == 0
    }
}
