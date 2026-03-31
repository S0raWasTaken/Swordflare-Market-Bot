use poise::serenity_prelude::Context as SerenityContext;
use tokio::time::interval;

use crate::database::supported_locale::SupportedLocale;
use crate::event_handler::confirm_flow::{
    ConfirmOutcome, await_both_confirmations, dm_cleanup,
};
use crate::{
    Res,
    database::{
        Data,
        auction_db::RunningAuction,
        trade_db::{Trade, TradeStatus},
    },
    magic_numbers::{DATABASE_CLEANUP_INTERVAL, TRADE_CONFIRMATION_TIMEOUT},
    post::{update_auction_post, update_post},
    print_err,
};
use poise::serenity_prelude as serenity;

pub async fn cleanup(ctx: SerenityContext, data: Data) {
    let mut interval = interval(DATABASE_CLEANUP_INTERVAL);
    loop {
        interval.tick().await;
        clean_database(&ctx, &data).await.inspect_err(print_err).ok();
    }
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
                update_post(ctx, data, id, trade.locale).await?;
            }
            status => {
                delete_post_message(ctx, data, trade, id, *status).await?;
            }
        }
    }

    // ── Running auctions ──────────────────────────────────────────────────────
    let auctions: Vec<(u64, RunningAuction)> = {
        let db = data.running_auctions.borrow_data()?;
        db.iter()
            .filter(|(_, a)| a.is_expired())
            .map(|(id, a)| (id, a.clone()))
            .collect()
    };

    for (id, auction) in auctions {
        let ctx = ctx.clone();
        let data = data.clone();
        tokio::spawn(async move {
            resolve_auction(&ctx, &data, id, auction)
                .await
                .inspect_err(print_err)
                .ok();
        });
    }

    Ok(())
}

/// Resolves an expired auction — offers to winner, falls back through bidders,
/// then moves it to the trades database regardless of outcome.
#[expect(clippy::too_many_lines)]
pub async fn resolve_auction(
    ctx: &SerenityContext,
    data: &Data,
    auction_id: u64,
    auction: RunningAuction,
) -> Res<()> {
    // Update both posts to show expired state before doing anything
    update_auction_post(ctx, data, auction_id, SupportedLocale::en_US)
        .await
        .inspect_err(print_err)
        .ok();
    update_auction_post(ctx, data, auction_id, SupportedLocale::ko_KR)
        .await
        .inspect_err(print_err)
        .ok();

    // Sort bidders highest to lowest
    let mut ranked_bidders: Vec<(serenity::UserId, u16)> =
        auction.bids.iter().map(|(&id, &amt)| (id, amt)).collect();
    ranked_bidders.sort_by_key(|(_, amt)| std::cmp::Reverse(*amt));

    let seller_id = auction.seller;
    let seller_locale =
        crate::database::supported_locale::get_user_locale(data, seller_id);

    let seller_user = match seller_id.to_user(ctx).await {
        Ok(u) => u,
        Err(e) => {
            print_err(&e);
            // Can't resolve - create trade with no winner and remove auction
            let trade = Trade::from((auction, None));
            data.trades.write(|db| db.insert(trade))?;
            data.trades.save()?;
            data.running_auctions.write(|db| db.remove(auction_id))?;
            data.running_auctions.save()?;
            return Ok(());
        }
    };

    let mut confirmed_winner = None;

    for (winner_id, winning_bid) in &ranked_bidders {
        let winner_locale = crate::database::supported_locale::get_user_locale(
            data, *winner_id,
        );

        let winner_user = match winner_id.to_user(ctx).await {
            Ok(u) => u,
            Err(e) => {
                print_err(&e);
                continue;
            }
        };

        let winner_dm = match winner_id.create_dm_channel(ctx).await {
            Ok(dm) => dm,
            Err(e) => {
                print_err(&e);
                continue;
            }
        };

        let winner_msg = match winner_dm
            .send_message(
                ctx,
                serenity::CreateMessage::default()
                    .content(t!(
                        "auction.resolve.winner_dm",
                        locale = winner_locale,
                        amount = winning_bid,
                        currency =
                            auction.currency_item.name.display(&winner_locale),
                        item = auction.item.name.display(&winner_locale),
                        quantity = auction.quantity,
                        server_link = &*crate::TRADING_SERVER_LINK,
                    ))
                    .components(vec![serenity::CreateActionRow::Buttons(
                        vec![
                            serenity::CreateButton::new(format!(
                                "confirm_buy_{auction_id}"
                            ))
                            .label(t!(
                                "buy.dm.button_confirm",
                                locale = winner_locale
                            ))
                            .style(serenity::ButtonStyle::Success),
                            serenity::CreateButton::new(format!(
                                "cancel_buy_{auction_id}"
                            ))
                            .label(t!(
                                "buy.dm.button_cancel",
                                locale = winner_locale
                            ))
                            .style(serenity::ButtonStyle::Danger),
                        ],
                    )]),
            )
            .await
        {
            Ok(m) => m,
            Err(e) => {
                print_err(&e);
                continue;
            }
        };

        let seller_dm = match seller_id.create_dm_channel(ctx).await {
            Ok(dm) => dm,
            Err(e) => {
                print_err(&e);
                data.trades
                    .write(|db| db.insert(Trade::from((auction, None))))?;
                data.trades.save()?;
                data.running_auctions.write(|db| db.remove(auction_id))?;
                data.running_auctions.save()?;
                return Ok(());
            }
        };

        let seller_msg = match seller_dm
            .send_message(
                ctx,
                serenity::CreateMessage::default()
                    .content(t!(
                        "auction.resolve.seller_dm",
                        locale = seller_locale,
                        winner = winner_user.name,
                        amount = winning_bid,
                        currency =
                            auction.currency_item.name.display(&seller_locale),
                        item = auction.item.name.display(&seller_locale),
                        quantity = auction.quantity,
                        server_link = &*crate::TRADING_SERVER_LINK,
                    ))
                    .components(vec![serenity::CreateActionRow::Buttons(
                        vec![
                            serenity::CreateButton::new(format!(
                                "confirm_sell_{auction_id}"
                            ))
                            .label(t!(
                                "buy.dm.button_confirm",
                                locale = seller_locale
                            ))
                            .style(serenity::ButtonStyle::Success),
                            serenity::CreateButton::new(format!(
                                "cancel_sell_{auction_id}"
                            ))
                            .label(t!(
                                "buy.dm.button_cancel",
                                locale = seller_locale
                            ))
                            .style(serenity::ButtonStyle::Danger),
                        ],
                    )]),
            )
            .await
        {
            Ok(m) => m,
            Err(e) => {
                print_err(&e);
                winner_msg.delete(&ctx.http).await.ok();
                continue;
            }
        };

        match await_both_confirmations(
            ctx,
            *winner_id,
            seller_id,
            auction_id,
            TRADE_CONFIRMATION_TIMEOUT,
            t!("buy.await.waiting_for_seller", locale = winner_locale)
                .into_owned(),
            t!("buy.await.waiting_for_buyer", locale = seller_locale)
                .into_owned(),
        )
        .await
        {
            ConfirmOutcome::BothConfirmed { buyer_int, seller_int } => {
                let buyer_content = t!(
                    "buy.done.buyer",
                    locale = winner_locale,
                    wants_total = winning_bid,
                    wants = auction.currency_item.name.display(&winner_locale),
                    seller = seller_user.name,
                    item_total = auction.quantity,
                    item = auction.item.name.display(&winner_locale),
                );
                let seller_content = t!(
                    "buy.done.seller",
                    locale = seller_locale,
                    wants_total = winning_bid,
                    wants = auction.currency_item.name.display(&seller_locale),
                    buyer = winner_user.name,
                    item_total = auction.quantity,
                    item = auction.item.name.display(&seller_locale),
                );

                buyer_int.create_response(ctx, serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::default()
                        .ephemeral(true).content(buyer_content)
                )).await.ok();
                seller_int.create_response(ctx, serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::default()
                        .ephemeral(true).content(seller_content)
                )).await.ok();

                dm_cleanup(ctx, &winner_msg, &seller_msg).await;

                confirmed_winner = Some((*winner_id, *winning_bid));
                break;
            }
            ConfirmOutcome::BuyerCancelled { buyer_int } => {
                buyer_int.create_response(ctx, serenity::CreateInteractionResponse::Message(
                        serenity::CreateInteractionResponseMessage::default()
                            .ephemeral(true)
                            .content(t!("buy.await.you_cancelled", locale = winner_locale)),
                    )).await.ok();
                seller_dm
                    .send_message(
                        ctx,
                        serenity::CreateMessage::default().content(t!(
                            "auction.resolve.winner_declined",
                            locale = seller_locale,
                            winner = winner_user.name
                        )),
                    )
                    .await
                    .ok();
                dm_cleanup(ctx, &winner_msg, &seller_msg).await;
            }
            ConfirmOutcome::SellerCancelled { seller_int } => {
                seller_int.create_response(ctx, serenity::CreateInteractionResponse::Message(
                        serenity::CreateInteractionResponseMessage::default()
                            .ephemeral(true)
                            .content(t!("buy.await.you_cancelled", locale = seller_locale)),
                    )).await.ok();
                winner_dm
                    .send_message(
                        ctx,
                        serenity::CreateMessage::default().content(t!(
                            "auction.resolve.seller_declined",
                            locale = winner_locale
                        )),
                    )
                    .await
                    .ok();
                dm_cleanup(ctx, &winner_msg, &seller_msg).await;
            }
            ConfirmOutcome::TimedOut => {
                dm_cleanup(ctx, &winner_msg, &seller_msg).await;
            }
        }
    }

    let trade = Trade::from((auction, confirmed_winner.map(|(id, _)| id)));

    data.trades.write(|db| db.insert(trade))?;
    data.trades.save()?;

    // Remove from running auctions
    data.running_auctions.write(|db| db.remove(auction_id))?;
    data.running_auctions.save()?;

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
