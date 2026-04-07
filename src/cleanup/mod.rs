use poise::serenity_prelude::Context as SerenityContext;
use tokio::time::interval;

use crate::cleanup::auction::resolve_auction;
use crate::{
    Res,
    database::{
        Data,
        trade_db::{Trade, TradeStatus},
    },
    magic_numbers::DATABASE_CLEANUP_INTERVAL,
    post::update_post,
    print_err,
};

pub mod auction;
pub mod dropguard;

pub async fn cleanup(ctx: SerenityContext, data: Data) {
    startup(&data).inspect_err(print_err).ok();

    let mut interval = interval(DATABASE_CLEANUP_INTERVAL);
    loop {
        interval.tick().await;
        clean_database(&ctx, &data).await.inspect_err(print_err).ok();
    }
}

pub fn startup(data: &Data) -> Res<()> {
    data.running_auctions.write(|db| {
        for (_, auction) in db.iter_mut() {
            auction.is_being_handled = false;
        }
    })?;
    Ok(())
}

pub async fn clean_database(ctx: &SerenityContext, data: &Data) -> Res<()> {
    // ── Trades ────────────────────────────────────────────────────────────────
    let trades: Vec<(u64, Trade)> = {
        let db = data.trades.borrow_data()?;
        db.iter().map(|(id, trade)| (id, trade.clone())).collect()
    };

    for (id, trade) in trades {
        match &trade.status() {
            TradeStatus::Running => {}
            TradeStatus::Timeout => {
                update_post(ctx, data, id, trade.locale)
                    .await
                    .inspect_err(print_err)
                    .ok();
            }
            status => {
                delete_post_message(ctx, data, trade, id, *status)
                    .await
                    .inspect_err(print_err)
                    .ok();
            }
        }
    }

    // ── Running auctions ──────────────────────────────────────────────────────
    let auctions = data.running_auctions.read(|db| {
        db.iter()
            .filter_map(|(id, a)| {
                (a.is_expired() && !a.is_being_handled).then_some(id)
            })
            .collect::<Vec<u64>>()
    })?;

    for id in auctions {
        let ctx = ctx.clone();
        let data = data.clone();
        tokio::spawn(async move {
            resolve_auction(&ctx, &data, id).await.inspect_err(print_err).ok();
        });
    }

    Ok(())
}

/// Will also delete from the database if it's marked as Invalid
async fn delete_post_message(
    ctx: &SerenityContext,
    data: &Data,
    mut trade: Trade,
    trade_id: u64,
    status: TradeStatus,
) -> Res<()> {
    trade.delete_messages(ctx, data).await?;

    if matches!(status, TradeStatus::Invalid) {
        let trades_db = &data.trades;
        trades_db.write(|db| db.remove(trade_id))?;
        trades_db.save()?;
    }
    Ok(())
}
