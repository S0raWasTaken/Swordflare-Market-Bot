use crate::Error;
use poise::serenity_prelude as serenity;

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

/// Waits for both buyer and seller to click confirm or cancel.
/// `buyer_waiting_msg` and `seller_waiting_msg` are sent as an immediate
/// ephemeral response to whoever confirms first, while waiting for the other.
pub async fn await_both_confirmations(
    ctx: &serenity::Context,
    buyer_id: serenity::UserId,
    seller_id: serenity::UserId,
    trade_id: u64,
    timeout: std::time::Duration,
    buyer_waiting_msg: String,
    seller_waiting_msg: String,
) -> ConfirmOutcome {
    let ctx1 = ctx.clone();
    let ctx2 = ctx.clone();

    let mut buyer_confirm = tokio::spawn(async move {
        serenity::collector::ComponentInteractionCollector::new(&ctx1)
            .author_id(buyer_id)
            .custom_ids(vec![
                format!("confirm_buy_{trade_id}"),
                format!("cancel_buy_{trade_id}"),
            ])
            .timeout(timeout)
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
            .timeout(timeout)
            .next()
            .await
            .ok_or::<Error>("Timed out".into())
    });

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
