use std::time::Duration;

// TODO: Discuss all magic numbers

/// Duration until a trade expires (2 days)
/// `trade.last_update.elapsed() > TRADE_EXPIRATION_TIME;`
pub const TRADE_EXPIRATION_TIME: Duration = Duration::from_hours(2 * 24);

/// Grace period for a [`TradeStatus::Timeout`] trade
/// to get either moderated, refreshed or set as
/// [`TradeStatus::Historical`]
pub const MODERATION_HOLD_PERIOD: Duration = Duration::from_hours(3 * 24);

/// Interval between cleanups in the database.
/// Found in `fn framework()`, in `main.rs`
pub const DATABASE_CLEANUP_INTERVAL: Duration = Duration::from_mins(10);

/// Period in which the DM buy/sell confirmation message will be up for.
/// This one won't survive a restart
pub const TRADE_CONFIRMATION_TIMEOUT: Duration = Duration::from_hours(24);
