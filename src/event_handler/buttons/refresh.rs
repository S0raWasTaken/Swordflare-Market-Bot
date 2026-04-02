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
    let refresh_ctx = ButtonContext::new(interaction, ctx, data, "refresh_");
    let locale = &refresh_ctx.locale();
    let not_seller = t!("refresh.error.not_seller", locale = locale);

    let error_condition = |seller| {
        if refresh_ctx.interaction_user_is_seller(seller) {
            None
        } else {
            Some(not_seller.to_string())
        }
    };

    let (trade_id, _) =
        break_or!(resolve_trade(&refresh_ctx, error_condition).await?);

    data.trades.write(|db| {
        db.get_mut(trade_id)
            .ok_or(t!("error.trade_not_found", locale = locale))?
            .refresh();
        Ok::<(), Error>(())
    })??;

    data.trades.save()?;

    update_posts(&refresh_ctx, trade_id).await?;

    refresh_ctx
        .reply_ephemeral(&t!("refresh.success", locale = locale))
        .await?;

    Ok(())
}
