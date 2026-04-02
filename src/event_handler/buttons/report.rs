use poise::serenity_prelude::{
    self as serenity, ComponentInteraction, CreateAllowedMentions,
    CreateInteractionResponse, CreateMessage, ModalInteraction,
};
use std::ops::ControlFlow::{Break, Continue};

use crate::{
    Res, break_or,
    database::Data,
    event_handler::buttons::{
        ButtonContext, ControlFlow, input_action_row, input_text,
        interaction_response, modal, modal_collector, parse_modal,
        resolve_trade,
    },
};

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn handle_report(
    ctx: &serenity::Context,
    interaction: &ComponentInteraction,
    data: &Data,
) -> Res<()> {
    let report_ctx = ButtonContext::new(interaction, ctx, data, "report_");
    let locale = &interaction.locale;
    let is_seller = t!("report.error.is_seller", locale = locale);

    let error_condition = |seller| {
        if report_ctx.interaction_user_is_seller(seller) {
            Some(is_seller.to_string())
        } else {
            None
        }
    };

    // Just check if seller is interaction.user and fail if so.
    let _ = break_or!(resolve_trade(&report_ctx, error_condition).await?);

    let (modal, report_text) =
        break_or!(prompt_report(&report_ctx, locale).await?);

    break_or!(
        log_and_save_report(&report_ctx, &modal, report_text, data, locale)
            .await?
    );

    modal
        .create_response(
            report_ctx.ctx,
            interaction_response(&t!("report.success", locale = locale), true),
        )
        .await?;

    Ok(())
}

// ── Steps ─────────────────────────────────────────────────────────────────────

async fn prompt_report(
    report_ctx: &ButtonContext<'_>,
    locale: &str,
) -> Res<ControlFlow<(ModalInteraction, String)>> {
    let custom_id = format!("write_report_{}", report_ctx.trade_id()?);

    report_ctx
        .create_response(CreateInteractionResponse::Modal(
            modal(&custom_id, &t!("report.modal.title", locale = locale))
                .components(vec![input_action_row(
                    input_text(
                        &t!("report.modal.label", locale = locale),
                        "report",
                        &t!("report.modal.placeholder", locale = locale),
                    )
                    .max_length(128),
                )]),
        ))
        .await?;

    let Some(modal) =
        modal_collector(report_ctx.ctx, report_ctx.user().id, custom_id).await
    else {
        return Ok(Break(()));
    };

    let parsed = parse_modal(
        &modal,
        t!("report.error.missing_input", locale = locale).to_string(),
    );

    let report_text = match parsed {
        Ok(mut text) => {
            if text.len() > 128
                && let Some((idx, _)) = text.char_indices().nth(128)
            {
                // We'll most likely never reach this,
                // but we can't just fully trust the api.
                text.truncate(idx);
            }
            text
        }
        Err(error) => {
            modal
                .create_response(
                    report_ctx.ctx,
                    interaction_response(&error, true),
                )
                .await?;
            return Ok(Break(()));
        }
    };

    if report_text.is_empty() {
        modal
            .create_response(
                report_ctx.ctx,
                interaction_response(
                    &t!("report.error.empty", locale = locale),
                    true,
                ),
            )
            .await?;
        return Ok(Break(()));
    }

    Ok(Continue((modal, report_text)))
}

pub async fn log_and_save_report(
    report_ctx: &ButtonContext<'_>,
    modal: &ModalInteraction,
    report: String,
    data: &Data,
    locale: &str,
) -> Res<ControlFlow<()>> {
    let reporter = report_ctx.user().id;
    let reports_channel = data.reports_channel;

    let (saved, post_link) = data.new_report(
        reporter,
        report.clone(),
        report_ctx.trade_id()?,
        locale,
    )?;

    if saved {
        let log_content = format!("<@{reporter}>: `{report}`\n{post_link}");
        reports_channel
            .send_message(
                report_ctx.ctx,
                CreateMessage::default().content(log_content).allowed_mentions(
                    CreateAllowedMentions::new()
                        .empty_roles()
                        .empty_users()
                        .everyone(false),
                ),
            )
            .await?;
        data.trades.save()?;
    } else {
        modal
            .create_response(
                report_ctx.ctx,
                interaction_response(
                    &t!("report.error.already_reported", locale = locale),
                    true,
                ),
            )
            .await?;
        return Ok(Break(()));
    }

    Ok(Continue(()))
}
