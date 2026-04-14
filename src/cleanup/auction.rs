use std::ops::ControlFlow::{Break, Continue};

use poise::serenity_prelude::{
    ButtonStyle, CacheHttp, Context as SerenityContext, CreateMessage, Message,
    PrivateChannel, User, UserId,
};

use crate::{
    Res, break_or,
    cleanup::dropguard::DropGuard,
    database::{
        Data,
        auction_db::RunningAuction,
        supported_locale::{SupportedLocale, get_user_locale},
        trade_db::Trade,
    },
    event_handler::{
        buttons::{
            ControlFlow, button, button_action_row, buy::TradeResult,
            interaction_response,
        },
        confirm_flow::{ConfirmOutcome, await_both_confirmations, dm_cleanup},
    },
    post::update_auction_post,
    print_err,
};

macro_rules! break_log {
    ($err:expr) => {
        match $err {
            Ok(ok) => ok,
            Err(e) => {
                print_err(&e);
                return Break(());
            }
        }
    };

    ($err:expr, $($smth_else:tt)*) => {
        match $err {
            Ok(ok) => ok,
            Err(e) => {
                print_err(&e);
                {
                    $($smth_else)*
                };
                return Break(());
            }
        }
    };
}

#[expect(clippy::type_complexity, reason = "I can't do much about it")]
fn fetch_auction<'a>(
    data: &'a Data,
    auction_id: u64,
) -> ControlFlow<(
    RunningAuction,
    DropGuard<&'a Data, impl Fn(&mut &'a Data) + 'a>,
)> {
    let Ok(auction) = data
        .running_auctions
        .write(|db| {
            let Some(auction) = db.get_mut(auction_id) else {
                return Break(());
            };

            if auction.is_being_handled {
                return Break(());
            }

            auction.is_being_handled = true;
            Continue(auction.clone())
        })
        .inspect_err(print_err)
    else {
        return Break(());
    };

    let auction = auction?;

    let handle_guard = DropGuard::new(data, move |data| {
        data.running_auctions
            .write(|db| {
                if let Some(auction) = db.get_mut(auction_id) {
                    auction.is_being_handled = false;
                }
            })
            .ok();
    });

    Continue((auction, handle_guard))
}

/// Resolves an expired auction — offers to winner, falls back through bidders,
/// then moves it to the trades database regardless of outcome.
pub async fn resolve_auction(
    ctx: &SerenityContext,
    data: &Data,
    auction_id: u64,
) -> Res<()> {
    let (auction, mut handle_guard) =
        break_or!(fetch_auction(data, auction_id));

    // Update both posts to show expired state before doing anything
    update_posts(ctx, data, auction_id).await.inspect_err(print_err).ok();

    let resolve_ctx = break_or!(
        ResolveContext::new(ctx, data, auction_id, auction.clone()).await?
    );

    let confirmed_winner = Box::pin(try_resolve(&resolve_ctx)).await;

    let trade = Trade::from((&auction, confirmed_winner.map(|(id, _)| id)));

    data.trades.write(|db| db.insert(trade))?;
    data.trades.save()?;

    // Remove from running auctions
    data.running_auctions.write(|db| db.remove(auction_id))?;
    data.running_auctions.save()?;

    handle_guard.disable();

    log_final_result(ctx, data, &auction, confirmed_winner.map(|w| w.0)).await
}

async fn try_resolve(
    resolve_ctx: &ResolveContext<'_>,
) -> Option<(UserId, u64)> {
    for (winner_id, winning_bid) in &resolve_ctx.ranked_bidders {
        let mut attempt_context =
            match AttemptContext::new(resolve_ctx, *winner_id, *winning_bid)
                .await
            {
                Continue(ctx) => ctx,
                Break(()) => continue,
            };

        let outcome = await_both_confirmations(
            resolve_ctx.ctx,
            *winner_id,
            resolve_ctx.seller_id,
            resolve_ctx.auction_id,
            (attempt_context.winner_locale, resolve_ctx.seller_locale),
            (&mut attempt_context.winner_msg, &mut attempt_context.seller_msg),
        )
        .await;

        let failed_attempt =
            match handle_outcome(resolve_ctx, attempt_context, outcome).await {
                Ok(success) => return Some(success),
                Err(failed) => failed,
            };

        log_attempts(
            resolve_ctx.ctx,
            resolve_ctx.data,
            failed_attempt,
            &resolve_ctx.auction,
            *winner_id,
        )
        .await
        .inspect_err(print_err)
        .ok();
    }
    None
}

#[expect(clippy::too_many_lines, reason = "I did my best lol")]
async fn handle_outcome(
    resolve_ctx: &ResolveContext<'_>,
    attempt_ctx: AttemptContext,
    outcome: ConfirmOutcome,
) -> Result<(UserId, u64), TradeResult> {
    let winner_locale = attempt_ctx.winner_locale;
    let winning_bid = attempt_ctx.winning_bid;
    let auction = &resolve_ctx.auction;
    let seller_locale = resolve_ctx.seller_locale;

    let fail = match outcome {
        ConfirmOutcome::BothConfirmed { buyer_int, seller_int } => {
            let buyer_content = t!(
                "buy.done.buyer",
                locale = winner_locale,
                wants_total = winning_bid,
                wants = auction.currency_item.display(winner_locale),
                seller = resolve_ctx.seller_user.name,
                item_total = auction.quantity,
                item = auction.item.display(winner_locale),
            );
            let seller_content = t!(
                "buy.done.seller",
                locale = seller_locale,
                wants_total = winning_bid,
                wants = auction.currency_item.display(seller_locale),
                buyer = attempt_ctx.winner_user.name,
                item_total = auction.quantity,
                item = auction.item.display(seller_locale),
            );

            buyer_int
                .create_response(
                    resolve_ctx,
                    interaction_response(&buyer_content, true),
                )
                .await
                .ok();
            seller_int
                .create_response(
                    resolve_ctx,
                    interaction_response(&seller_content, true),
                )
                .await
                .ok();

            dm_cleanup(
                resolve_ctx.ctx,
                &attempt_ctx.winner_msg,
                &attempt_ctx.seller_msg,
            )
            .await;

            return Ok((attempt_ctx.winner_user.id, attempt_ctx.winning_bid));
        }
        ConfirmOutcome::BuyerCancelled { buyer_int } => {
            buyer_int
                .create_response(
                    resolve_ctx,
                    interaction_response(
                        &t!("buy.await.you_cancelled", locale = winner_locale),
                        true,
                    ),
                )
                .await
                .ok();
            resolve_ctx
                .seller_dm
                .send_message(
                    resolve_ctx,
                    CreateMessage::default().content(t!(
                        "auction.resolve.winner_declined",
                        locale = seller_locale,
                        winner = attempt_ctx.winner_user.name
                    )),
                )
                .await
                .ok();
            dm_cleanup(
                resolve_ctx.ctx,
                &attempt_ctx.winner_msg,
                &attempt_ctx.seller_msg,
            )
            .await;
            TradeResult::BuyerCancelled
        }
        ConfirmOutcome::SellerCancelled { seller_int } => {
            seller_int
                .create_response(
                    resolve_ctx,
                    interaction_response(
                        &t!("buy.await.you_cancelled", locale = seller_locale),
                        true,
                    ),
                )
                .await
                .ok();
            attempt_ctx
                .winner_dm
                .send_message(
                    resolve_ctx,
                    CreateMessage::default().content(t!(
                        "auction.resolve.seller_declined",
                        locale = winner_locale
                    )),
                )
                .await
                .ok();
            dm_cleanup(
                resolve_ctx.ctx,
                &attempt_ctx.winner_msg,
                &attempt_ctx.seller_msg,
            )
            .await;
            TradeResult::SellerCancelled
        }
        ConfirmOutcome::TimedOut => {
            dm_cleanup(
                resolve_ctx.ctx,
                &attempt_ctx.winner_msg,
                &attempt_ctx.seller_msg,
            )
            .await;
            TradeResult::TimedOut
        }
    };
    Err(fail)
}

async fn update_posts(
    ctx: &SerenityContext,
    data: &Data,
    auction_id: u64,
) -> Res<()> {
    update_auction_post(ctx, data, auction_id, SupportedLocale::en_US).await?;
    update_auction_post(ctx, data, auction_id, SupportedLocale::ko_KR).await
}

async fn log_final_result(
    ctx: &SerenityContext,
    data: &Data,
    auction: &RunningAuction,
    winner: Option<UserId>,
) -> Res<()> {
    let winner_display =
        winner.map_or("None".to_string(), |w| format!("<@{w}>"));
    let auction_message = make_auction_message(data, auction, &winner_display)?;

    let message = if winner.is_none() {
        format!("Auction failed ─ {auction_message}")
    } else {
        format!("Auction successful ─ {auction_message}")
    };

    data.log(ctx, &message).await
}

async fn log_attempts(
    ctx: &SerenityContext,
    data: &Data,
    failed_attempt: TradeResult,
    auction: &RunningAuction,
    current_winner: UserId,
) -> Res<()> {
    let auction_message =
        make_auction_message(data, auction, &format!("<@{current_winner}>"))?;

    let message = match failed_attempt {
        TradeResult::BuyerCancelled => {
            format!("Auction attempt cancelled by bidder ─ {auction_message}",)
        }
        TradeResult::SellerCancelled => {
            format!("Auction attempt cancelled by seller ─ {auction_message}",)
        }
        TradeResult::TimedOut => {
            format!("Auction attempt timed out ─ {auction_message}",)
        }
        TradeResult::Confirmed => unreachable!(),
    };

    data.log(ctx, &message).await
}

fn make_auction_message(
    data: &Data,
    auction: &RunningAuction,
    current_winner: &str,
) -> Res<String> {
    let auction_display = auction.display_simple("en-US");
    let seller = auction.seller;
    let link = auction.message_link(SupportedLocale::en_US, data)?;

    Ok(format!(
        "seller: <@{seller}>, bidder: {current_winner}, auction: {auction_display}\n\
            {link}"
    ))
}

fn fail_resolution_early(
    data: &Data,
    auction: &RunningAuction,
    auction_id: u64,
) -> Res<()> {
    let trade = Trade::from((auction, None));
    data.trades.write(|db| db.insert(trade))?;
    data.trades.save()?;
    data.running_auctions.write(|db| db.remove(auction_id))?;
    data.running_auctions.save()?;
    Ok(())
}

struct ResolveContext<'a> {
    ctx: &'a SerenityContext,
    data: &'a Data,
    auction_id: u64,
    auction: RunningAuction,
    ranked_bidders: Vec<(UserId, u64)>,
    seller_id: UserId,
    seller_locale: &'static str,
    seller_user: User,
    seller_dm: PrivateChannel,
}

impl<'a> ResolveContext<'a> {
    pub async fn new(
        ctx: &'a SerenityContext,
        data: &'a Data,
        auction_id: u64,
        auction: RunningAuction,
    ) -> Res<ControlFlow<Self>> {
        // Sort bidders highest to lowest
        let mut ranked_bidders: Vec<(UserId, u64)> =
            auction.bids.iter().map(|(&id, &amt)| (id, amt)).collect();
        ranked_bidders.sort_by_key(|(_, amt)| std::cmp::Reverse(*amt));

        let seller_id = auction.seller;
        let seller_locale = get_user_locale(ctx, data, seller_id).await;

        let seller_user = match seller_id.to_user(ctx).await {
            Ok(u) => u,
            Err(e) => {
                print_err(&e);
                fail_resolution_early(data, &auction, auction_id)?;
                return Ok(Break(()));
            }
        };

        let seller_dm = match seller_id.create_dm_channel(ctx).await {
            Ok(dm) => dm,
            Err(e) => {
                print_err(&e);
                fail_resolution_early(data, &auction, auction_id)?;
                return Ok(Break(()));
            }
        };

        Ok(Continue(Self {
            ctx,
            data,
            auction_id,
            auction,
            ranked_bidders,
            seller_id,
            seller_locale,
            seller_user,
            seller_dm,
        }))
    }
}

impl CacheHttp for ResolveContext<'_> {
    fn http(&self) -> &poise::serenity_prelude::Http {
        self.ctx.http()
    }
}

struct AttemptContext {
    winner_locale: &'static str,
    winner_user: User,
    winner_dm: PrivateChannel,
    winner_msg: Message,
    winning_bid: u64,
    seller_msg: Message,
}

impl AttemptContext {
    pub async fn new(
        resolve_ctx: &ResolveContext<'_>,
        winner_id: UserId,
        winning_bid: u64,
    ) -> ControlFlow<Self> {
        let winner_locale =
            get_user_locale(resolve_ctx, resolve_ctx.data, winner_id).await;
        let seller_locale = resolve_ctx.seller_locale;
        let auction = &resolve_ctx.auction;

        let winner_user = break_log!(winner_id.to_user(resolve_ctx).await);

        let winner_dm =
            break_log!(winner_id.create_dm_channel(resolve_ctx).await);

        let winner_msg = match winner_dm
            .send_message(
                resolve_ctx,
                CreateMessage::default()
                    .content(t!(
                        "auction.resolve.winner_dm",
                        locale = winner_locale,
                        amount = winning_bid,
                        currency = resolve_ctx
                            .auction
                            .currency_item
                            .display(winner_locale),
                        item = resolve_ctx.auction.item.display(winner_locale),
                        quantity = resolve_ctx.auction.quantity,
                        server_link = &*crate::TRADING_SERVER_LINK,
                    ))
                    .components(vec![button_action_row(vec![
                        button(
                            format!("confirm_buy_{}", resolve_ctx.auction_id),
                            t!("buy.dm.button_confirm", locale = winner_locale),
                            ButtonStyle::Success,
                        ),
                        button(
                            format!("cancel_buy_{}", resolve_ctx.auction_id),
                            t!("buy.dm.button_cancel", locale = winner_locale),
                            ButtonStyle::Danger,
                        ),
                    ])]),
            )
            .await
        {
            Ok(m) => m,
            Err(e) => {
                print_err(&e);
                return Break(());
            }
        };

        let seller_msg = resolve_ctx
            .seller_dm
            .send_message(
                resolve_ctx,
                CreateMessage::default()
                    .content(t!(
                        "auction.resolve.seller_dm",
                        locale = seller_locale,
                        winner = winner_user.name,
                        amount = winning_bid,
                        currency = auction.currency_item.display(seller_locale),
                        item = auction.item.display(seller_locale),
                        quantity = auction.quantity,
                        server_link = &*crate::TRADING_SERVER_LINK,
                    ))
                    .components(vec![button_action_row(vec![
                        button(
                            format!("confirm_sell_{}", resolve_ctx.auction_id),
                            t!("buy.dm.button_confirm", locale = seller_locale),
                            ButtonStyle::Success,
                        ),
                        button(
                            format!("cancel_sell_{}", resolve_ctx.auction_id),
                            t!("buy.dm.button_cancel", locale = seller_locale),
                            ButtonStyle::Danger,
                        ),
                    ])]),
            )
            .await;

        let seller_msg =
            break_log!(seller_msg, winner_msg.delete(resolve_ctx).await.ok());

        Continue(Self {
            winner_locale,
            winner_user,
            winner_dm,
            winner_msg,
            winning_bid,
            seller_msg,
        })
    }
}
