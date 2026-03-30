use poise::serenity_prelude::Context as SerenityContext;
use tokio::time::interval;

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

pub async fn cleanup(ctx: SerenityContext, data: Data) {
    let mut interval = interval(DATABASE_CLEANUP_INTERVAL);
    loop {
        interval.tick().await;
        clean_database(&ctx, &data).await.inspect_err(print_err).ok();
    }
}

pub async fn clean_database(ctx: &SerenityContext, data: &Data) -> Res<()> {
    let trades: Vec<(u64, Trade)> = {
        let db = data.trades.borrow_data()?;

        db.iter().map(|(id, trade)| (id, trade.clone())).collect()
    };

    for (id, trade) in trades {
        match trade.status() {
            TradeStatus::Running => {}
            TradeStatus::Timeout => {
                update_post(ctx, data, id, trade.locale).await?;
            }
            status => {
                delete_post_message(ctx, data, trade, id, status).await?;
            }
        }
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
