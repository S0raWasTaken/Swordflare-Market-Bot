use std::ops::ControlFlow::{Break, Continue};

use poise::serenity_prelude::{
    self as serenity, ButtonStyle, ComponentInteraction, CreateActionRow,
    CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage,
};

use crate::{
    Error, Res, break_or,
    database::{Data, auction_db::RunningAuction},
    event_handler::buttons::{
        ButtonContext, ControlFlow, button, button_action_row,
    },
    magic_numbers::TRADE_CONFIRMATION_TIMEOUT,
};

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn handle_auction_cancel(
    ctx: &serenity::Context,
    interaction: &ComponentInteraction,
    data: &Data,
) -> Res<()> {
    let mut cancel_ctx =
        ButtonContext::new(interaction, ctx, data, "au_cancel_").await;
    let locale = &interaction.locale;

    let auction =
        break_or!(check_auction_ownership(&cancel_ctx, locale).await?);

    let interaction = break_or!(confirm_cancel(&cancel_ctx, locale).await?);

    cancel_ctx.interaction = &interaction;
    cancel_ctx.prefix = "au_confirm_cancel_";

    log_and_cancel(&cancel_ctx, locale, data, auction).await
}

// ── Steps ─────────────────────────────────────────────────────────────────────

async fn check_auction_ownership(
    cancel_ctx: &ButtonContext<'_>,
    locale: &str,
) -> Res<ControlFlow<RunningAuction>> {
    let Some(auction) = cancel_ctx.data.running_auctions.read(|db| {
        Ok::<Option<RunningAuction>, Error>(
            db.get(cancel_ctx.trade_id()?).cloned(),
        )
    })??
    else {
        cancel_ctx
            .reply_ephemeral(&t!("cancel.error.not_found", locale = locale))
            .await?;
        return Ok(Break(()));
    };

    if auction.seller != cancel_ctx.user().id {
        cancel_ctx
            .reply_ephemeral(&t!("cancel.error.not_seller", locale = locale))
            .await?;
        return Ok(Break(()));
    }

    Ok(Continue(auction))
}

async fn confirm_cancel(
    cancel_ctx: &ButtonContext<'_>,
    locale: &str,
) -> Res<ControlFlow<ComponentInteraction>> {
    let auction_id = cancel_ctx.trade_id()?;
    let embed = CreateEmbed::default()
        .title(t!("cancel.embed.title", locale = locale))
        .description(t!("cancel.embed.description", locale = locale));

    cancel_ctx
        .create_response(response(message(
            embed,
            cancel_buttons(cancel_ctx.trade_id()?, locale),
        )))
        .await?;

    let response = cancel_ctx.interaction.get_response(cancel_ctx.ctx).await?;

    let Some(component) = response
        .await_component_interaction(cancel_ctx.ctx)
        .author_id(cancel_ctx.user().id)
        .timeout(TRADE_CONFIRMATION_TIMEOUT)
        .await
    else {
        return Ok(Break(()));
    };

    let not_confirmed =
        component.data.custom_id != format!("au_confirm_cancel_{auction_id}");

    if not_confirmed {
        component
            .create_response(
                cancel_ctx.ctx,
                CreateInteractionResponse::Acknowledge,
            )
            .await?;
        return Ok(Break(()));
    }

    Ok(Continue(component))
}

async fn log_and_cancel(
    cancel_ctx: &ButtonContext<'_>,
    locale: &str,
    data: &Data,
    auction: RunningAuction,
) -> Res<()> {
    let auction_id = cancel_ctx.trade_id()?;
    let user = cancel_ctx.user().id;

    let auction_display = auction.display_simple("en-US"); // Log locale
    let highest_bid =
        auction.highest_bid().map_or("None".to_string(), |(user, bid)| {
            format!("<@{user}>: `{bid}`")
        });

    auction.delete_messages(cancel_ctx.ctx, data).await?;

    data.running_auctions.write(|db| {
        db.remove(auction_id)
            .ok_or(t!("cancel.error.not_found", locale = locale))
    })??;

    data.running_auctions.save()?;

    let log_content = format!(
        "<@{user}> cancelled their auction: {auction_display}\n\
        highest bid: {highest_bid}"
    );

    data.log(cancel_ctx.ctx, &log_content).await?;

    cancel_ctx.reply_ephemeral(&t!("cancel.success", locale = locale)).await?;
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn cancel_buttons(auction_id: u64, locale: &str) -> CreateActionRow {
    button_action_row(vec![
        button(
            format!("au_confirm_cancel_{auction_id}"),
            t!("buy.confirm.button_confirm", locale = locale),
            ButtonStyle::Danger,
        ),
        button(
            format!("au_deny_cancel_{auction_id}"),
            t!("buy.confirm.button_cancel", locale = locale),
            ButtonStyle::Secondary,
        ),
    ])
}

fn response(
    message: CreateInteractionResponseMessage,
) -> CreateInteractionResponse {
    CreateInteractionResponse::Message(message)
}

fn message(
    embed: CreateEmbed,
    buttons: CreateActionRow,
) -> CreateInteractionResponseMessage {
    CreateInteractionResponseMessage::default()
        .ephemeral(true)
        .embed(embed)
        .components(vec![buttons])
}
