use poise::serenity_prelude::Context as SerenityContext;

use crate::{
    database::{
        Data,
        trade_db::{Trade, TradeStatus},
    },
    post::update_post,
    print_err,
};

pub async fn cleanup(ctx: &SerenityContext, data: &Data) {
    let trades: Vec<(u64, Trade)> = {
        let Ok(db) = data.trades.borrow_data().inspect_err(print_err) else {
            return;
        };
        db.iter().map(|(id, trade)| (id, trade.clone())).collect()
    };

    for (id, trade) in trades {
        match trade.status() {
            TradeStatus::Running => {}
            TradeStatus::Timeout => {
                update_post(ctx, data, id, trade.locale)
                    .await
                    .inspect_err(print_err)
                    .ok();
            }
            status => {
                delete_post_message(ctx, data, trade, id, status).await;
            }
        }
    }
}

/// Will also delete from the database if it's marked as Invalid
async fn delete_post_message(
    ctx: &SerenityContext,
    data: &Data,
    mut trade: Trade,
    trade_id: u64,
    status: TradeStatus,
) {
    trade.delete_messages(ctx, data).await.inspect_err(print_err).ok();

    if matches!(status, TradeStatus::Invalid) {
        let trades_db = &data.trades;
        trades_db.write(|db| db.remove(trade_id)).inspect_err(print_err).ok();
        trades_db.save().inspect_err(print_err).ok();
    }
}
