use crate::{
    Error,
    event_handler::buttons::{button, button_action_row},
    magic_numbers::TRADE_CONFIRMATION_TIMEOUT,
};
use poise::serenity_prelude::{
    self as serenity, ButtonStyle, CreateActionRow, EditMessage, Message,
};

pub async fn dm_cleanup(
    ctx: &serenity::Context,
    buyer_msg: &serenity::Message,
    seller_msg: &serenity::Message,
) {
    let _ =
        tokio::join!(buyer_msg.delete(&ctx.http), seller_msg.delete(&ctx.http));
}

pub enum ConfirmOutcome {
    BothConfirmed {
        buyer_int: Box<serenity::ComponentInteraction>,
        seller_int: Box<serenity::ComponentInteraction>,
    },
    BuyerCancelled {
        buyer_int: Box<serenity::ComponentInteraction>,
    },
    SellerCancelled {
        seller_int: Box<serenity::ComponentInteraction>,
    },
    TimedOut,
}

fn disabled_buttons(locale: &str) -> Vec<CreateActionRow> {
    vec![button_action_row(vec![
        button(
            "empty_",
            t!("buy.dm.button_confirm", locale = locale),
            ButtonStyle::Success,
        )
        .disabled(true),
        button(
            "_empty",
            t!("buy.dm.button_cancel", locale = locale),
            ButtonStyle::Danger,
        )
        .disabled(true),
    ])]
}

/// Waits for both buyer and seller to click confirm or cancel.
/// `buyer_waiting_msg` and `seller_waiting_msg` are sent as an immediate
/// ephemeral response to whoever confirms first, while waiting for the other.
pub async fn await_both_confirmations(
    ctx: &serenity::Context,
    buyer_id: serenity::UserId,
    seller_id: serenity::UserId,
    trade_id: u64,
    locales: (&str, &str), // (buyer, seller)
    (buyer_msg, seller_msg): (&mut Message, &mut Message),
) -> ConfirmOutcome {
    let ctx1 = ctx.clone();
    let ctx2 = ctx.clone();

    let buyer_waiting_msg =
        t!("buy.await.waiting_for_seller", locale = locales.0).into_owned();
    let seller_waiting_msg =
        t!("buy.await.waiting_for_buyer", locale = locales.1).into_owned();

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

    let new_buyer_msg = EditMessage::new()
        .content(buyer_msg.content.clone())
        .components(disabled_buttons(locales.0));

    let new_seller_msg = EditMessage::new()
        .content(seller_msg.content.clone())
        .components(disabled_buttons(locales.1));

    tokio::select! {
        result = &mut buyer_confirm => {
            let Ok(Ok(buyer_int)) = result else {
                seller_confirm.abort();
                return ConfirmOutcome::TimedOut;
            };
            let buyer_int = Box::new(buyer_int);

            if !buyer_int.data.custom_id.starts_with("confirm_buy_") {
                seller_confirm.abort();
                return ConfirmOutcome::BuyerCancelled { buyer_int };
            }

            buyer_msg.edit(ctx, new_buyer_msg).await.ok();

            // Respond immediately so the interaction doesn't hang
            buyer_int.create_response(ctx, serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::default()
                    .ephemeral(true)
                    .content(buyer_waiting_msg),
            )).await.ok();

            match seller_confirm.await {
                Ok(Ok(seller_int)) if seller_int.data.custom_id.starts_with("confirm_sell_") => {
                    ConfirmOutcome::BothConfirmed { buyer_int, seller_int: Box::new(seller_int) }
                }
                Ok(Ok(seller_int)) => ConfirmOutcome::SellerCancelled { seller_int: Box::new(seller_int) },
                _ => ConfirmOutcome::TimedOut,
            }
        }

        result = &mut seller_confirm => {
            let Ok(Ok(seller_int)) = result else {
                buyer_confirm.abort();
                return ConfirmOutcome::TimedOut;
            };
            let seller_int = Box::new(seller_int);

            if !seller_int.data.custom_id.starts_with("confirm_sell_") {
                buyer_confirm.abort();
                return ConfirmOutcome::SellerCancelled { seller_int };
            }

            seller_msg.edit(ctx, new_seller_msg).await.ok();

            // Respond immediately so the interaction doesn't hang
            seller_int.create_response(ctx, serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::default()
                    .ephemeral(true)
                    .content(seller_waiting_msg),
            )).await.ok();

            match buyer_confirm.await {
                Ok(Ok(buyer_int)) if buyer_int.data.custom_id.starts_with("confirm_buy_") => {
                    ConfirmOutcome::BothConfirmed { buyer_int: Box::new(buyer_int), seller_int }
                }
                Ok(Ok(buyer_int)) => ConfirmOutcome::BuyerCancelled { buyer_int: Box::new(buyer_int) },
                _ => ConfirmOutcome::TimedOut,
            }
        }
    }
}
