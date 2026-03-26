use crate::{Error, Res};
use poise::serenity_prelude::{self as serenity, CacheHttp, CreateMessage};
use std::time::Duration;

const CONFIRMATION_TIMEOUT: Duration = Duration::from_hours(24);

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
    let trade_id: u64 = interaction
        .data
        .custom_id
        .strip_prefix("buy_")
        .ok_or("Invalid custom_id")?
        .parse()?;

    let (seller_id, stock, item, item_quantity, wants, wanted_amount) = {
        let db = data.trades.borrow_data()?;
        let trade = db.get(trade_id).ok_or("Trade not found")?;
        (
            trade.seller,
            trade.stock,
            trade.item,
            trade.quantity,
            trade.wants,
            trade.wanted_amount,
        )
    };

    if buyer.id == seller_id {
        interaction
            .create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::default()
                        .ephemeral(true)
                        .content("❌ You can't buy your own trade."),
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
    }))
}

/// Shows the lots modal, waits for submission, validates input.
/// Returns None if the user timed out.
async fn prompt_lots(
    ctx: &serenity::Context,
    interaction: &serenity::ComponentInteraction,
    trade_ctx: &TradeContext,
) -> Res<Option<(u16, serenity::ModalInteraction)>> {
    interaction
        .create_response(
            ctx,
            serenity::CreateInteractionResponse::Modal(
                serenity::CreateModal::new(
                    format!("quantity_{}", trade_ctx.trade_id),
                    "How many lots do you want?",
                )
                .components(vec![
                    serenity::CreateActionRow::InputText(
                        serenity::CreateInputText::new(
                            serenity::InputTextStyle::Short,
                            "Lots",
                            "quantity",
                        )
                        .min_length(1)
                        .max_length(5)
                        .placeholder("Enter number of lots"),
                    ),
                ]),
            ),
        )
        .await?;

    let Some(modal) = serenity::collector::ModalInteractionCollector::new(ctx)
        .author_id(interaction.user.id)
        .custom_ids(vec![format!("quantity_{}", trade_ctx.trade_id)])
        .timeout(CONFIRMATION_TIMEOUT)
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
        .ok_or("Missing lots input")
        .and_then(|v| v.parse::<u16>().map_err(|_| "Invalid number"));

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
            .create_response(ctx, serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::default()
                    .ephemeral(true)
                    .content(format!(
                        "❌ Invalid amount. There are only {} lot(s) remaining.",
                        trade_ctx.stock
                    )),
            ))
            .await?;
        return Ok(None);
    }

    Ok(Some((lots, modal)))
}

/// Shows a confirmation embed summarising what the buyer is about to do.
/// Returns false if they cancelled or timed out.
async fn confirm_purchase(
    ctx: &serenity::Context,
    modal: &serenity::ModalInteraction,
    trade_ctx: &TradeContext,
    lots: u16,
) -> Res<bool> {
    let embed = serenity::CreateEmbed::default()
        .title("Confirm Purchase")
        .description(format!(
            "You're buying **{lots}** lot(s) from **{}**.",
            trade_ctx.seller_name
        ))
        .field(
            "You will receive",
            format!(
                "**{}** x{}",
                trade_ctx.item.name,
                u32::from(trade_ctx.item_quantity) * u32::from(lots)
            ),
            true,
        )
        .field(
            "You will give",
            format!(
                "**{}** x{}",
                trade_ctx.wants.name,
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
                            .label("Confirm")
                            .style(serenity::ButtonStyle::Success),
                            serenity::CreateButton::new(format!(
                                "cancel_purchase_{}",
                                trade_ctx.trade_id
                            ))
                            .label("Cancel")
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
        .timeout(CONFIRMATION_TIMEOUT)
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
                        .content("Check your DMs!")
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
                        .content("❌ Purchase cancelled.")
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
        ..
    } = trade_ctx;

    let buyer_dm = buyer.id.create_dm_channel(ctx).await?;
    let buyer_msg = buyer_dm
        .send_message(
            ctx,
            CreateMessage::default()
                .content(format!(
                    "You're about to buy **x{} {}** in exchange for **x{} {}** \
                ({lots} lot(s)) from **{seller_name}**.\n\
                Go find them in-game and confirm once the trade is done.",
                    item_quantity * lots,
                    item.name,
                    wanted_amount * lots,
                    wants.name,
                ))
                .components(vec![serenity::CreateActionRow::Buttons(vec![
                    serenity::CreateButton::new(format!(
                        "confirm_buy_{trade_id}"
                    ))
                    .label("Confirm Trade Done")
                    .style(serenity::ButtonStyle::Success),
                    serenity::CreateButton::new(format!(
                        "cancel_buy_{trade_id}"
                    ))
                    .label("Cancel")
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

    let content = format!(
        "**{}** wants to buy **x{} {}** from you in exchange for **x{} {}** \
        ({lots} lot(s)).\nGo find them in-game and confirm once done.",
        buyer.name,
        item_quantity * lots,
        item.name,
        wanted_amount * lots,
        wants.name,
    );

    let components = vec![serenity::CreateActionRow::Buttons(vec![
        serenity::CreateButton::new(format!("confirm_sell_{trade_id}"))
            .label("Confirm Trade Done")
            .style(serenity::ButtonStyle::Success),
        serenity::CreateButton::new(format!("cancel_sell_{trade_id}"))
            .label("Cancel")
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
    let TradeContext { trade_id, seller_id, seller_name, .. } = trade_ctx;
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
            .timeout(CONFIRMATION_TIMEOUT)
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
            .timeout(CONFIRMATION_TIMEOUT)
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
                        .ephemeral(true).content("⚠️ You cancelled the trade."),
                )).await?;
                seller_dm.send_message(ctx, serenity::CreateMessage::default()
                    .content(format!("⚠️ **{}** cancelled the trade.", buyer.name))
                ).await?;
                cleanup(ctx, buyer_msg, seller_msg).await;
                return Ok(());
            }

            buyer_int.create_response(ctx, serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::default()
                    .ephemeral(true).content("✅ Got it! Waiting for the seller to confirm..."),
            )).await?;

            let seller_int = seller_confirm.await??;
            if !seller_int.data.custom_id.starts_with("confirm_sell_") {
                seller_int.create_response(ctx, serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::default()
                        .ephemeral(true).content("⚠️ You cancelled the trade."),
                )).await?;
                buyer_int.create_followup(ctx, serenity::CreateInteractionResponseFollowup::default()
                    .ephemeral(true).content(format!("⚠️ **{seller_name}** cancelled the trade."))
                ).await?;
                cleanup(ctx, buyer_msg, seller_msg).await;
                return Ok(());
            }

            finish_trade(ctx, data, trade_ctx, pending, &buyer_int, &seller_int, true).await?;
        }

        result = &mut seller_confirm => {
            let seller_int = result??;
            if !seller_int.data.custom_id.starts_with("confirm_sell_") {
                seller_int.create_response(ctx, serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::default()
                        .ephemeral(true).content("⚠️ You cancelled the trade."),
                )).await?;
                buyer_dm.send_message(ctx, serenity::CreateMessage::default()
                    .content(format!("⚠️ **{seller_name}** cancelled the trade."))
                ).await?;
                cleanup(ctx, buyer_msg, seller_msg).await;
                return Ok(());
            }

            seller_int.create_response(ctx, serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::default()
                    .ephemeral(true).content("✅ Got it! Waiting for the buyer to confirm..."),
            )).await?;

            let buyer_int = buyer_confirm.await??;
            if !buyer_int.data.custom_id.starts_with("confirm_buy_") {
                buyer_int.create_response(ctx, serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::default()
                        .ephemeral(true).content("⚠️ You cancelled the trade."),
                )).await?;
                seller_int.create_followup(ctx, serenity::CreateInteractionResponseFollowup::default()
                    .ephemeral(true).content(format!("⚠️ **{}** cancelled the trade.", buyer.name))
                ).await?;
                cleanup(ctx, buyer_msg, seller_msg).await;
                return Ok(());
            }

            finish_trade(ctx, data, trade_ctx, pending, &buyer_int, &seller_int, false).await?;
        }
    }

    Ok(())
}

/// Deletes both DM messages.
async fn cleanup(
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

    update_or_delete_post(ctx, data, trade_ctx).await?;
    cleanup(ctx, buyer_msg, seller_msg).await;

    let buyer_content = format!(
        "✅ Trade confirmed! You gave **x{} {}** to **{seller_name}** and received **x{} {}**. Thanks for trading!",
        wanted_amount * quantity,
        wants.name,
        item_quantity * quantity,
        item.name,
    );
    let seller_content = if is_sold_out {
        format!(
            "✅ Trade confirmed! You gave **x{} {}** and received **x{} {}** from **{}** — all stock sold, post removed.",
            item_quantity * quantity,
            item.name,
            wanted_amount * quantity,
            wants.name,
            buyer.name,
        )
    } else {
        format!(
            "✅ Trade confirmed! You gave **x{} {}** and received **x{} {}** from **{}**. Stock decremented.",
            item_quantity * quantity,
            item.name,
            wanted_amount * quantity,
            wants.name,
            buyer.name,
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

/// Edits the trade post to reflect new stock, or deletes it if sold out.
async fn update_or_delete_post(
    ctx: &serenity::Context,
    data: &crate::Data,
    trade_ctx: &TradeContext,
) -> Res<()> {
    let (message_id, trade) = {
        let db = data.trades.borrow_data()?;
        match db.get(trade_ctx.trade_id) {
            Some(t) => match t.message_id {
                Some(mid) => (mid, t.clone()),
                None => return Ok(()),
            },
            None => return Ok(()),
        }
    };

    if trade.is_sold_out() {
        data.trade_posting_channel
            .delete_message(ctx.http(), message_id)
            .await?;
        return Ok(());
    }

    let seller = trade.seller.to_user(ctx).await?;
    let avatar_url =
        seller.avatar_url().unwrap_or_else(|| seller.default_avatar_url());

    let embed = serenity::CreateEmbed::default()
        .title(format!("Trade by {}", seller.name))
        .thumbnail(avatar_url)
        .field(
            "Offering",
            format!(
                "**{}** x{} ({:?})",
                trade.item.name, trade.quantity, trade.item.rarity
            ),
            true,
        )
        .field(
            "Wants",
            format!(
                "**{}** x{} ({:?})",
                trade.wants.name, trade.wanted_amount, trade.wants.rarity
            ),
            true,
        )
        .field("Stock", format!("{} lot(s) remaining", trade.stock), true)
        .color(serenity::Color::GOLD);

    data.trade_posting_channel
        .edit_message(
            ctx.http(),
            message_id,
            serenity::EditMessage::default().embed(embed).components(vec![
                serenity::CreateActionRow::Buttons(vec![
                    serenity::CreateButton::new(format!(
                        "buy_{}",
                        trade_ctx.trade_id
                    ))
                    .label("Buy")
                    .style(serenity::ButtonStyle::Success),
                ]),
            ]),
        )
        .await?;

    Ok(())
}
