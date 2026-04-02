use poise::serenity_prelude::{self as serenity, ComponentInteraction};

use crate::{
    Error, Res, break_or,
    database::Data,
    event_handler::buttons::{ButtonContext, resolve_trade, update_posts},
};

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn handle_refresh(
    ctx: &serenity::Context,
    interaction: &ComponentInteraction,
    data: &Data,
) -> Res<()> {
    let button_context = ButtonContext::new(interaction, ctx, data, "refresh_");
    let locale = &button_context.locale();

    let not_seller = t!("refresh.error.not_seller", locale = locale);

    let (trade_id, _) =
        break_or!(resolve_trade(&button_context, &not_seller).await?);

    data.trades.write(|db| {
        db.get_mut(trade_id)
            .ok_or(t!("error.trade_not_found", locale = locale))?
            .refresh();
        Ok::<(), Error>(())
    })??;

    update_posts(&button_context, trade_id).await?;

    button_context
        .reply_ephemeral(&t!("refresh.success", locale = locale))
        .await?;

    Ok(())
}
