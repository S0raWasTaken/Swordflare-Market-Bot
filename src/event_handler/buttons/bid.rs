use std::ops::ControlFlow::{Break, Continue};

use poise::serenity_prelude::{
    self as serenity, CacheHttp, ComponentInteraction,
    CreateInteractionResponse, CreateMessage, Message, ModalInteraction,
    UserId,
};

use crate::{
    Res, break_or,
    database::{
        Data,
        supported_locale::{SupportedLocale, get_user_locale},
    },
    event_handler::buttons::{
        ButtonContext, ControlFlow, input_action_row, input_text,
        interaction_response, modal, modal_collector, parse_number_in_modal,
    },
    items::Item,
    post::update_auction_post,
    print_err,
};

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn handle_bid(
    ctx: &serenity::Context,
    interaction: &ComponentInteraction,
    data: &Data,
) -> Res<()> {
    let bid_ctx = ButtonContext::new(interaction, ctx, data, "bid_").await;

    let (auction_id, min_next_bid, currency_name) =
        break_or!(resolve_auction(&bid_ctx).await?);

    let (amount, modal) = break_or!(
        prompt_bid(&bid_ctx, auction_id, min_next_bid, &currency_name).await?
    );

    if let Some(outbid_ctx) = break_or!(
        place_bid(
            &bid_ctx,
            &modal,
            auction_id,
            amount,
            min_next_bid,
            &currency_name
        )
        .await?
    ) {
        dm_old_highest(&bid_ctx, outbid_ctx).await.inspect_err(print_err).ok();
    }

    finish(&bid_ctx, &modal, auction_id, amount, &currency_name).await
}

// ── Steps ─────────────────────────────────────────────────────────────────────

type AuctionId = u64;
type MinNextBid = u64;

async fn resolve_auction(
    bid_ctx: &ButtonContext<'_>,
) -> Res<ControlFlow<(AuctionId, MinNextBid, String)>> {
    let locale = bid_ctx.locale();
    let auction_id = bid_ctx.trade_id()?;

    let (seller_id, is_expired, min_next_bid, currency_name) = {
        let db = bid_ctx.data.running_auctions.borrow_data()?;
        let auction = db
            .get(auction_id)
            .ok_or(t!("error.trade_not_found", locale = locale))?;
        (
            auction.seller,
            auction.is_expired(),
            auction.min_next_bid(bid_ctx.user().id),
            auction.currency_item.display(locale),
        )
    };

    if bid_ctx.user().id == seller_id {
        bid_ctx
            .reply_ephemeral(&t!("auction.error.self_bid", locale = locale))
            .await?;
        return Ok(Break(()));
    }

    if is_expired {
        bid_ctx
            .reply_ephemeral(&t!("auction.error.expired", locale = locale))
            .await?;
        return Ok(Break(()));
    }

    Ok(Continue((auction_id, min_next_bid, currency_name)))
}

type Amount = u64;

async fn prompt_bid(
    bid_ctx: &ButtonContext<'_>,
    auction_id: u64,
    min_next_bid: u64,
    currency_name: &str,
) -> Res<ControlFlow<(Amount, ModalInteraction)>> {
    let locale = bid_ctx.locale();
    let custom_id = format!("bid_amount_{auction_id}");

    bid_ctx
        .create_response(CreateInteractionResponse::Modal(
            modal(&custom_id, &t!("auction.modal.title", locale = locale))
                .components(vec![input_action_row(input_text(
                    &t!(
                        "auction.modal.input_label",
                        locale = locale,
                        min = min_next_bid,
                        currency = currency_name
                    ),
                    "bid_amount",
                    &min_next_bid.to_string(),
                ))]),
        ))
        .await?;

    let Some(modal) =
        modal_collector(bid_ctx.ctx, bid_ctx.user().id, custom_id).await
    else {
        return Ok(Break(()));
    };

    let amount = match parse_number_in_modal(
        &modal,
        locale,
        t!("auction.error.missing_bid_input", locale = locale).to_string(),
    ) {
        Ok(a) => a,
        Err(e) => {
            modal
                .create_response(
                    bid_ctx.ctx,
                    interaction_response(&e.to_string(), true),
                )
                .await?;
            return Ok(Break(()));
        }
    };

    Ok(Continue((amount, modal)))
}

async fn place_bid(
    bid_ctx: &ButtonContext<'_>,
    modal: &ModalInteraction,
    auction_id: u64,
    amount: u64,
    min_next_bid: u64,
    currency_name: &str,
) -> Res<ControlFlow<Option<OutbidContext>>> {
    let locale = bid_ctx.locale();
    let bidder_id = bid_ctx.user().id;

    let (bid_accepted, outbid_ctx) =
        bid_ctx.data.running_auctions.write(|db| {
            let Some(auction) = db.get_mut(auction_id) else {
                return (false, None);
            };
            if !auction.is_valid_bid(bidder_id, amount) {
                return (false, None);
            }

            let old_highest = auction.highest_bid().and_then(
                |(highest_bidder, highest_amount)| {
                    (amount > highest_amount && highest_bidder != bidder_id)
                        .then_some(OutbidContext {
                            outbid_bidder: highest_bidder,
                            outbid_bid: highest_amount,
                            new_highest_bidder: bidder_id,
                            highest_bid: amount,
                            currency_name: auction.currency_item,
                        })
                },
            );

            auction.insert(bidder_id, amount);
            (true, old_highest)
        })?;

    if !bid_accepted {
        let current_min = bid_ctx
            .data
            .running_auctions
            .borrow_data()?
            .get(auction_id)
            .map_or(min_next_bid, |au| au.min_next_bid(bidder_id));

        modal
            .create_response(
                bid_ctx.ctx,
                interaction_response(
                    &t!(
                        "auction.error.invalid_bid",
                        locale = locale,
                        min = current_min,
                        currency = currency_name
                    ),
                    true,
                ),
            )
            .await?;
        return Ok(Break(()));
    }

    bid_ctx.data.running_auctions.save()?;

    Ok(Continue(outbid_ctx))
}

async fn dm_old_highest(
    bid_ctx: &ButtonContext<'_>,
    outbid_ctx: OutbidContext,
) -> serenity::Result<Message> {
    let locale =
        get_user_locale(bid_ctx.ctx, bid_ctx.data, outbid_ctx.outbid_bidder)
            .await;

    outbid_ctx
        .dm(
            bid_ctx.ctx,
            t!(
                "auction.outbid",
                user = outbid_ctx.new_highest_bidder,
                old_bid = outbid_ctx.outbid_bid,
                currency_name = outbid_ctx.currency_name.display(locale),
                new_bid_diff = outbid_ctx.bid_diff(),
                locale = locale
            ),
        )
        .await
}

async fn finish(
    bid_ctx: &ButtonContext<'_>,
    modal: &ModalInteraction,
    auction_id: u64,
    amount: u64,
    currency_name: &str,
) -> Res<()> {
    let locale = bid_ctx.locale();

    modal
        .create_response(
            bid_ctx.ctx,
            interaction_response(
                &t!(
                    "auction.bid.accepted",
                    locale = locale,
                    amount = amount,
                    currency = currency_name
                ),
                true,
            ),
        )
        .await?;

    let (en_result, ko_result) = tokio::join! {
        update_auction_post(bid_ctx.ctx, bid_ctx.data, auction_id, SupportedLocale::en_US),
        update_auction_post(bid_ctx.ctx, bid_ctx.data, auction_id, SupportedLocale::ko_KR)
    };

    en_result?;
    ko_result?;

    Ok(())
}

// ── Data Types ────────────────────────────────────────────────────────────────

struct OutbidContext {
    outbid_bidder: UserId,
    outbid_bid: u64,
    new_highest_bidder: UserId,
    highest_bid: u64,
    currency_name: Item,
}

impl OutbidContext {
    pub fn bid_diff(&self) -> u64 {
        self.highest_bid.saturating_sub(self.outbid_bid)
    }

    pub async fn dm(
        &self,
        cache_http: impl CacheHttp,
        msg: impl Into<String>,
    ) -> serenity::Result<Message> {
        self.outbid_bidder
            .direct_message(cache_http, CreateMessage::default().content(msg))
            .await
    }
}
