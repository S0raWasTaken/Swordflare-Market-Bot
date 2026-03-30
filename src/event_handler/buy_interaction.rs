use crate::{
    Error, Res, TRADING_SERVER_LINK,
    database::supported_locale::{SupportedLocale, get_user_locale},
    magic_numbers::TRADE_CONFIRMATION_TIMEOUT,
    post::update_post,
};
use poise::serenity_prelude::{self as serenity, CreateMessage};

// ── Data types ────────────────────────────────────────────────────────────────

struct TradeContext {
    trade_id: u64,
    seller_id: serenity::UserId,
    seller_name: String,
    stock: u16,
    item: crate::items::Item,
    item_quantity: u16,
    wants: crate::items::Item,
    wanted_amount: u16,
    buyer_locale: String,
    seller_locale: String,
}

struct PendingTrade<'a> {
    buyer_dm: serenity::PrivateChannel,
    seller_dm: serenity::PrivateChannel,
    buyer_msg: serenity::Message,
    seller_msg: serenity::Message,
    buyer: &'a serenity::User,
    lots: u16,
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn handle_buy_interaction(
    ctx: &serenity::Context,
    interaction: &serenity::ComponentInteraction,
    data: &crate::Data,
) -> Res<()> {
    let buyer = &interaction.user;

    let Some(trade_ctx) = resolve_trade(ctx, interaction, data, buyer).await?
    else {
        return Ok(());
    };

    let Some((lots, modal)) = prompt_lots(ctx, interaction, &trade_ctx).await?
    else {
        return Ok(());
    };

    if !confirm_purchase(ctx, &modal, &trade_ctx, lots).await? {
        return Ok(());
    }

    let pending = send_trade_dms(ctx, &trade_ctx, buyer, lots).await?;

    await_confirmations(ctx, data, &trade_ctx, &pending).await?;

    Ok(())
}

// ── Steps ─────────────────────────────────────────────────────────────────────

/// Parses the trade ID, fetches trade data, and guards against self-purchase.
/// Returns None if the interaction was handled (e.g. self-buy rejection) and
/// the caller should return early.
async fn resolve_trade(
    ctx: &serenity::Context,
    interaction: &serenity::ComponentInteraction,
    data: &crate::Data,
    buyer: &serenity::User,
) -> Res<Option<TradeContext>> {
    let buyer_locale = get_user_locale(data, buyer.id);
    let trade_id: u64 = interaction
        .data
        .custom_id
        .strip_prefix("buy_")
        .ok_or(t!("error.invalid_custom_id", locale = buyer_locale))?
        .parse()?;

    let (seller_id, stock, item, item_quantity, wants, wanted_amount) = {
        let db = data.trades.borrow_data()?;
        let trade = db
            .get(trade_id)
            .ok_or(t!("error.trade_not_found", locale = buyer_locale))?;
        (
            trade.seller,
            trade.stock,
            trade.item,
            trade.quantity,
            trade.wants,
            trade.wanted_amount,
        )
    };
    let seller_locale = get_user_locale(data, seller_id);

    if buyer.id == seller_id {
        interaction
            .create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::default()
                        .ephemeral(true)
                        .content(t!(
                            "buy.error.self_buy",
                            locale = buyer_locale
                        )),
                ),
            )
            .await?;
        return Ok(None);
    }

    let seller_name = seller_id.to_user(ctx).await?.name;

    Ok(Some(TradeContext {
        trade_id,
        seller_id,
        seller_name,
        stock,
        item,
        item_quantity,
        wants,
        wanted_amount,
        buyer_locale,
        seller_locale,
    }))
}

/// Shows the lots modal, waits for submission, validates input.
/// Returns None if the user timed out.
async fn prompt_lots(
    ctx: &serenity::Context,
    interaction: &serenity::ComponentInteraction,
    trade_ctx: &TradeContext,
) -> Res<Option<(u16, serenity::ModalInteraction)>> {
    let buyer_locale = &trade_ctx.buyer_locale;
    interaction
        .create_response(
            ctx,
            serenity::CreateInteractionResponse::Modal(
                serenity::CreateModal::new(
                    format!("quantity_{}", trade_ctx.trade_id),
                    t!("buy.modal.title", locale = buyer_locale),
                )
                .components(vec![
                    serenity::CreateActionRow::InputText(
                        serenity::CreateInputText::new(
                            serenity::InputTextStyle::Short,
                            t!("buy.modal.input_label", locale = buyer_locale),
                            "quantity",
                        )
                        .min_length(1)
                        .max_length(5)
                        .placeholder(t!(
                            "buy.modal.input_placeholder",
                            locale = buyer_locale
                        )),
                    ),
                ]),
            ),
        )
        .await?;

    let Some(modal) = serenity::collector::ModalInteractionCollector::new(ctx)
        .author_id(interaction.user.id)
        .custom_ids(vec![format!("quantity_{}", trade_ctx.trade_id)])
        .timeout(TRADE_CONFIRMATION_TIMEOUT)
        .next()
        .await
    else {
        return Ok(None);
    };

    let parsed = modal
        .data
        .components
        .iter()
        .flat_map(|r| r.components.iter())
        .find_map(|c| {
            if let serenity::ActionRowComponent::InputText(t) = c {
                t.value.as_deref()
            } else {
                None
            }
        })
        .ok_or(t!("error.missing_lots_input", locale = buyer_locale))
        .and_then(|v| {
            v.parse::<u16>()
                .map_err(|_| t!("error.invalid_number", locale = buyer_locale))
        });

    let lots = match parsed {
        Ok(q) => q,
        Err(e) => {
            modal
                .create_response(
                    ctx,
                    serenity::CreateInteractionResponse::Message(
                        serenity::CreateInteractionResponseMessage::default()
                            .ephemeral(true)
                            .content(format!("❌ {e}")),
                    ),
                )
                .await?;
            return Ok(None);
        }
    };

    if lots == 0 || lots > trade_ctx.stock {
        modal
            .create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::default()
                        .ephemeral(true)
                        .content(t!(
                            "buy.error.invalid_amount",
                            locale = buyer_locale,
                            stock = trade_ctx.stock
                        )),
                ),
            )
            .await?;
        return Ok(None);
    }

    Ok(Some((lots, modal)))
}

/// Shows a confirmation embed summarising what the buyer is about to do.
/// Returns false if they cancelled or timed out.
#[expect(clippy::too_many_lines)]
async fn confirm_purchase(
    ctx: &serenity::Context,
    modal: &serenity::ModalInteraction,
    trade_ctx: &TradeContext,
    lots: u16,
) -> Res<bool> {
    let buyer_locale = &trade_ctx.buyer_locale;
    let embed = serenity::CreateEmbed::default()
        .title(t!("buy.confirm.title", locale = buyer_locale))
        .description(t!(
            "buy.confirm.description",
            locale = buyer_locale,
            lots = lots,
            seller = trade_ctx.seller_name
        ))
        .field(
            t!("buy.confirm.field_receive", locale = buyer_locale),
            format!(
                "**{}** x{}",
                trade_ctx.item.name.display(buyer_locale),
                u32::from(trade_ctx.item_quantity) * u32::from(lots)
            ),
            true,
        )
        .field(
            t!("buy.confirm.field_give", locale = buyer_locale),
            format!(
                "**{}** x{}",
                trade_ctx.wants.name.display(buyer_locale),
                u32::from(trade_ctx.wanted_amount) * u32::from(lots)
            ),
            true,
        )
        .color(serenity::Color::GOLD);

    modal
        .create_response(
            ctx,
            serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::default()
                    .ephemeral(true)
                    .embed(embed)
                    .components(vec![serenity::CreateActionRow::Buttons(
                        vec![
                            serenity::CreateButton::new(format!(
                                "confirm_purchase_{}",
                                trade_ctx.trade_id
                            ))
                            .label(t!(
                                "buy.confirm.button_confirm",
                                locale = buyer_locale
                            ))
                            .style(serenity::ButtonStyle::Success),
                            serenity::CreateButton::new(format!(
                                "cancel_purchase_{}",
                                trade_ctx.trade_id
                            ))
                            .label(t!(
                                "buy.confirm.button_cancel",
                                locale = buyer_locale
                            ))
                            .style(serenity::ButtonStyle::Danger),
                        ],
                    )]),
            ),
        )
        .await?;

    let modal_msg = modal.get_response(ctx).await?;
    let Some(component) = modal_msg
        .await_component_interaction(ctx)
        .author_id(modal.user.id)
        .timeout(TRADE_CONFIRMATION_TIMEOUT)
        .await
    else {
        return Ok(false);
    };

    let confirmed = component.data.custom_id
        == format!("confirm_purchase_{}", trade_ctx.trade_id);

    if confirmed {
        component
            .create_response(
                ctx,
                serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::default()
                        .content(t!(
                            "buy.confirm.check_dms",
                            locale = buyer_locale
                        ))
                        .embeds(vec![])
                        .components(vec![]),
                ),
            )
            .await?;
    } else {
        component
            .create_response(
                ctx,
                serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::default()
                        .content(t!(
                            "buy.confirm.cancelled",
                            locale = buyer_locale
                        ))
                        .embeds(vec![])
                        .components(vec![]),
                ),
            )
            .await?;
    }

    Ok(confirmed)
}

/// Sends DMs to both buyer and seller with trade details and confirm/cancel buttons.
async fn send_trade_dms<'a>(
    ctx: &serenity::Context,
    trade_ctx: &TradeContext,
    buyer: &'a serenity::User,
    lots: u16,
) -> Res<PendingTrade<'a>> {
    let TradeContext {
        trade_id,
        seller_id,
        seller_name,
        item,
        item_quantity,
        wants,
        wanted_amount,
        buyer_locale,
        seller_locale,
        ..
    } = trade_ctx;

    // Confirmed to be set in `fn main()`
    let private_server_link = &*TRADING_SERVER_LINK;

    let buyer_dm = buyer.id.create_dm_channel(ctx).await?;
    let buyer_msg = buyer_dm
        .send_message(
            ctx,
            CreateMessage::default()
                .content(t!(
                    "buy.dm.buyer",
                    locale = buyer_locale,
                    item_total = item_quantity * lots,
                    item = item.name.display(buyer_locale),
                    wants_total = wanted_amount * lots,
                    wants = wants.name.display(buyer_locale),
                    lots = lots,
                    seller = seller_name,
                    server_link = private_server_link
                ))
                .components(vec![serenity::CreateActionRow::Buttons(vec![
                    serenity::CreateButton::new(format!(
                        "confirm_buy_{trade_id}"
                    ))
                    .label(t!("buy.dm.button_confirm", locale = buyer_locale))
                    .style(serenity::ButtonStyle::Success),
                    serenity::CreateButton::new(format!(
                        "cancel_buy_{trade_id}"
                    ))
                    .label(t!("buy.dm.button_cancel", locale = buyer_locale))
                    .style(serenity::ButtonStyle::Danger),
                ])]),
        )
        .await?;

    let seller_dm = match seller_id.create_dm_channel(ctx).await {
        Ok(dm) => dm,
        Err(e) => {
            buyer_msg.delete(&ctx.http).await.ok();
            return Err(e.into());
        }
    };

    let content = t!(
        "buy.dm.seller",
        locale = seller_locale,
        buyer = buyer.name,
        item_total = item_quantity * lots,
        item = item.name.display(seller_locale),
        wants_total = wanted_amount * lots,
        wants = wants.name.display(seller_locale),
        lots = lots,
        server_link = private_server_link
    );

    let components = vec![serenity::CreateActionRow::Buttons(vec![
        serenity::CreateButton::new(format!("confirm_sell_{trade_id}"))
            .label(t!("buy.dm.button_confirm", locale = seller_locale))
            .style(serenity::ButtonStyle::Success),
        serenity::CreateButton::new(format!("cancel_sell_{trade_id}"))
            .label(t!("buy.dm.button_cancel", locale = seller_locale))
            .style(serenity::ButtonStyle::Danger),
    ])];

    let message =
        CreateMessage::default().content(content).components(components);

    let seller_msg = match seller_dm.send_message(ctx, message).await {
        Ok(msg) => msg,
        Err(e) => {
            buyer_msg.delete(&ctx.http).await.ok();
            return Err(e.into());
        }
    };

    Ok(PendingTrade { buyer_dm, seller_dm, buyer_msg, seller_msg, buyer, lots })
}

/// Waits for both parties to confirm or cancel, then finalises or aborts.
#[expect(clippy::too_many_lines)]
async fn await_confirmations(
    ctx: &serenity::Context,
    data: &crate::Data,
    trade_ctx: &TradeContext,
    pending: &PendingTrade<'_>,
) -> Res<()> {
    let TradeContext {
        trade_id,
        seller_id,
        seller_name,
        buyer_locale,
        seller_locale,
        ..
    } = trade_ctx;
    let PendingTrade {
        buyer,
        buyer_dm,
        seller_dm,
        buyer_msg,
        seller_msg,
        lots: _,
    } = pending;

    let ctx1 = ctx.clone();
    let ctx2 = ctx.clone();
    let buyer_id = buyer.id;
    let trade_id = *trade_id;
    let seller_id = *seller_id;

    let mut buyer_confirm = tokio::spawn(async move {
        serenity::collector::ComponentInteractionCollector::new(&ctx1)
            .author_id(buyer_id)
            .custom_ids(vec![
                format!("confirm_buy_{trade_id}"),
                format!("cancel_buy_{trade_id}"),
            ])
            .timeout(TRADE_CONFIRMATION_TIMEOUT)
            .next()
            .await
            .ok_or::<Error>("Timed out".into())
    });

    let mut seller_confirm = tokio::spawn(async move {
        serenity::collector::ComponentInteractionCollector::new(&ctx2)
            .author_id(seller_id)
            .custom_ids(vec![
                format!("confirm_sell_{trade_id}"),
                format!("cancel_sell_{trade_id}"),
            ])
            .timeout(TRADE_CONFIRMATION_TIMEOUT)
            .next()
            .await
            .ok_or::<Error>("Timed out".into())
    });

    tokio::select! {
        result = &mut buyer_confirm => {
            let buyer_int = result??;
            if !buyer_int.data.custom_id.starts_with("confirm_buy_") {
                buyer_int.create_response(ctx, serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::default()
                        .ephemeral(true).content(t!("buy.await.you_cancelled", locale = buyer_locale)),
                )).await?;
                seller_dm.send_message(ctx, serenity::CreateMessage::default()
                    .content(t!("buy.await.buyer_cancelled", locale = seller_locale, name = buyer.name))
                ).await?;
                dm_cleanup(ctx, buyer_msg, seller_msg).await;
                return Ok(());
            }

            buyer_int.create_response(ctx, serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::default()
                    .ephemeral(true).content(t!("buy.await.waiting_for_seller", locale = buyer_locale)),
            )).await?;

            let seller_int = seller_confirm.await??;
            if !seller_int.data.custom_id.starts_with("confirm_sell_") {
                seller_int.create_response(ctx, serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::default()
                        .ephemeral(true).content(t!("buy.await.you_cancelled", locale = seller_locale)),
                )).await?;
                buyer_int.create_followup(ctx, serenity::CreateInteractionResponseFollowup::default()
                    .ephemeral(true).content(t!("buy.await.seller_cancelled", locale = buyer_locale, name = seller_name))
                ).await?;
                dm_cleanup(ctx, buyer_msg, seller_msg).await;
                return Ok(());
            }

            finish_trade(ctx, data, trade_ctx, pending, &buyer_int, &seller_int, true).await?;
        }

        result = &mut seller_confirm => {
            let seller_int = result??;
            if !seller_int.data.custom_id.starts_with("confirm_sell_") {
                seller_int.create_response(ctx, serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::default()
                        .ephemeral(true).content(t!("buy.await.you_cancelled", locale = seller_locale)),
                )).await?;
                buyer_dm.send_message(ctx, serenity::CreateMessage::default()
                    .content(t!("buy.await.seller_cancelled", locale = buyer_locale, name = seller_name))
                ).await?;
                dm_cleanup(ctx, buyer_msg, seller_msg).await;
                return Ok(());
            }

            seller_int.create_response(ctx, serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::default()
                    .ephemeral(true).content(t!("buy.await.waiting_for_buyer", locale = seller_locale)),
            )).await?;

            let buyer_int = buyer_confirm.await??;
            if !buyer_int.data.custom_id.starts_with("confirm_buy_") {
                buyer_int.create_response(ctx, serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::default()
                        .ephemeral(true).content(t!("buy.await.you_cancelled", locale = buyer_locale)),
                )).await?;
                seller_int.create_followup(ctx, serenity::CreateInteractionResponseFollowup::default()
                    .ephemeral(true).content(t!("buy.await.buyer_cancelled", locale = seller_locale, name = buyer.name))
                ).await?;
                dm_cleanup(ctx, buyer_msg, seller_msg).await;
                return Ok(());
            }

            finish_trade(ctx, data, trade_ctx, pending, &buyer_int, &seller_int, false).await?;
        }
    }

    Ok(())
}

/// Deletes both DM messages.
async fn dm_cleanup(
    ctx: &serenity::Context,
    buyer_msg: &serenity::Message,
    seller_msg: &serenity::Message,
) {
    buyer_msg.delete(&ctx.http).await.ok();
    seller_msg.delete(&ctx.http).await.ok();
}

// ── Finalisation ──────────────────────────────────────────────────────────────

async fn finish_trade(
    ctx: &serenity::Context,
    data: &crate::Data,
    trade_ctx: &TradeContext,
    pending: &PendingTrade<'_>,
    buyer_int: &serenity::ComponentInteraction,
    seller_int: &serenity::ComponentInteraction,
    buyer_confirmed_first: bool,
) -> Res<()> {
    let TradeContext {
        trade_id,
        seller_name,
        item,
        item_quantity,
        wants,
        wanted_amount,
        buyer_locale,
        seller_locale,
        ..
    } = trade_ctx;
    let PendingTrade { buyer, buyer_msg, seller_msg, lots, .. } = pending;
    let quantity = *lots;

    let is_sold_out = data.trades.write(|db| {
        if let Some(trade) = db.get_mut(*trade_id) {
            trade.stock = trade.stock.saturating_sub(quantity);
            trade.buyers.insert(buyer.id);
            trade.is_sold_out()
        } else {
            false
        }
    })?;
    data.trades.save()?;

    update_post(ctx, data, *trade_id, SupportedLocale::en_US).await?;
    update_post(ctx, data, *trade_id, SupportedLocale::ko_KR).await?;
    dm_cleanup(ctx, buyer_msg, seller_msg).await;

    let buyer_content = t!(
        "buy.done.buyer",
        locale = buyer_locale,
        wants_total = wanted_amount * quantity,
        wants = wants.name.display(buyer_locale),
        seller = seller_name,
        item_total = item_quantity * quantity,
        item = item.name.display(buyer_locale),
    );
    let seller_content = if is_sold_out {
        t!(
            "buy.done.seller_sold_out",
            locale = seller_locale,
            wants_total = wanted_amount * quantity,
            wants = wants.name.display(seller_locale),
            buyer = buyer.name,
            item_total = item_quantity * quantity,
            item = item.name.display(seller_locale),
        )
    } else {
        t!(
            "buy.done.seller",
            locale = seller_locale,
            wants_total = wanted_amount * quantity,
            wants = wants.name.display(seller_locale),
            buyer = buyer.name,
            item_total = item_quantity * quantity,
            item = item.name.display(seller_locale),
        )
    };

    if buyer_confirmed_first {
        buyer_int
            .create_followup(
                ctx,
                serenity::CreateInteractionResponseFollowup::default()
                    .ephemeral(true)
                    .content(buyer_content),
            )
            .await?;
        seller_int
            .create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::default()
                        .ephemeral(true)
                        .content(seller_content),
                ),
            )
            .await?;
    } else {
        buyer_int
            .create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::default()
                        .ephemeral(true)
                        .content(buyer_content),
                ),
            )
            .await?;
        seller_int
            .create_followup(
                ctx,
                serenity::CreateInteractionResponseFollowup::default()
                    .ephemeral(true)
                    .content(seller_content),
            )
            .await?;
    }

    Ok(())
}
