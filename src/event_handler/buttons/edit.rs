use std::ops::ControlFlow::{Break, Continue};

use poise::serenity_prelude::{
    self as serenity, ComponentInteraction, CreateInteractionResponse,
    ModalInteraction,
};

use crate::{
    Error, Res, break_or,
    database::{Data, trade_db::Trade},
    event_handler::buttons::{
        ButtonContext, ControlFlow, input_action_row, input_text,
        interaction_response, modal, modal_collector, parse_input_modal,
        resolve_trade, update_posts,
    },
};

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn handle_edit(
    ctx: &serenity::Context,
    interaction: &ComponentInteraction,
    data: &Data,
) -> Res<()> {
    let edit_ctx = ButtonContext::new(interaction, ctx, data, "edit_");
    let not_seller = t!("edit.error.not_seller", locale = &edit_ctx.locale());

    let (trade_id, trade) =
        break_or!(resolve_trade(&edit_ctx, &not_seller).await?);
    let (lots, modal) = break_or!(prompt_edit(&edit_ctx, &trade).await?);

    update_trade(&edit_ctx, trade_id, lots).await?;

    finish(&edit_ctx, &modal).await
}

// ── Steps ─────────────────────────────────────────────────────────────────────

type Lots = u16;
async fn prompt_edit(
    edit_ctx: &ButtonContext<'_>,
    trade: &Trade,
) -> Res<ControlFlow<(Lots, ModalInteraction)>> {
    let locale = &edit_ctx.locale();
    let custom_id = format!("quantity_{}", edit_ctx.trade_id()?);

    edit_ctx
        .create_response(CreateInteractionResponse::Modal(
            modal(&custom_id, &t!("edit.modal.title", locale = locale))
                .components(vec![input_action_row(input_text(
                    &t!("edit.modal.input_label", locale = locale),
                    "quantity",
                    &t!("edit.modal.placeholder", locale = locale),
                ))]),
        ))
        .await?;

    let Some(modal) =
        modal_collector(edit_ctx.ctx, edit_ctx.user().id, custom_id).await
    else {
        return Ok(Break(()));
    };

    let parsed = parse_input_modal(
        &modal,
        locale,
        t!("edit.error.missing_stock_input", locale = locale).to_string(),
    );

    let lots = match parsed {
        Ok(stock) => stock / trade.quantity,
        Err(e) => {
            modal
                .create_response(
                    edit_ctx.ctx,
                    interaction_response(&e.to_string(), true),
                )
                .await?;
            return Ok(Break(()));
        }
    };

    // lots == 0 is valid, it means the seller is out of stock.

    Ok(Continue((lots, modal)))
}

async fn update_trade(
    edit_ctx: &ButtonContext<'_>,
    trade_id: u64,
    lots: u16,
) -> Res<()> {
    let data = edit_ctx.data;

    data.trades.write(|db| {
        let trade = db
            .get_mut(trade_id)
            .ok_or(format!("Trade not found: {trade_id}"))?;

        trade.stock = lots;
        trade.refresh();
        Ok::<(), Error>(())
    })??;

    update_posts(edit_ctx, trade_id).await?;

    Ok(())
}

async fn finish(
    edit_ctx: &ButtonContext<'_>,
    modal: &ModalInteraction,
) -> Res<()> {
    edit_ctx.data.trades.save()?;

    Ok(modal
        .create_response(
            edit_ctx.ctx,
            interaction_response(
                &t!("edit.success", locale = &edit_ctx.locale()),
                true,
            ),
        )
        .await?)
}
