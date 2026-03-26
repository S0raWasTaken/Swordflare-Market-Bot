use crate::database::trade_db::{EXPIRATION_TIME, Trade};
use crate::{Context, Res, items::ITEMS};
use poise::{CreateReply, serenity_prelude as serenity};
use std::time::Duration;

const CONFIRM_TIMEOUT: Duration = Duration::from_mins(1);

#[expect(clippy::unused_async)]
async fn autocomplete_item<'a>(
    _ctx: Context<'_>,
    partial: &'a str,
) -> impl Iterator<Item = String> + 'a {
    ITEMS
        .iter()
        .filter(move |i| {
            i.name.to_lowercase().contains(&partial.to_lowercase())
        })
        .map(|i| i.name.to_string())
}

#[expect(clippy::too_many_lines)]
#[poise::command(slash_command)]
pub async fn new_trade(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_item"] trading_item: String,
    trade_quantity: u16,
    #[autocomplete = "autocomplete_item"] for_item: String,
    wants_amount: u16,
    stock: u16,
) -> Res<()> {
    let item = ITEMS
        .iter()
        .find(|i| i.name == trading_item)
        .ok_or("Invalid item name")?;
    let wants =
        ITEMS.iter().find(|i| i.name == for_item).ok_or("Invalid item name")?;

    if trade_quantity == 0 {
        ctx.send(
            CreateReply::default()
                .content("❌ Trade quantity must be greater than zero.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    // stock means total items; lots = how many times the trade can be done
    let lots = stock / trade_quantity;
    if lots == 0 {
        ctx.send(
            CreateReply::default()
                .content(format!(
                    "❌ Stock ({stock}) must be at least equal to trade quantity ({trade_quantity})."
                ))
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    let seller = ctx.author();
    let avatar_url =
        seller.avatar_url().unwrap_or_else(|| seller.default_avatar_url());

    // Step 1: Show confirmation
    let confirm_embed = serenity::CreateEmbed::default()
        .title("Confirm Trade Post")
        .description(format!(
            "You're about to sell a total of **x{} {}** across **{lots}** lot(s) of x{trade_quantity} each.",
            lots * trade_quantity,
            item.name,
        ))
        .thumbnail(avatar_url.clone())
        .field(
            "Offering (per lot)",
            format!("**{}** x{} ({:?})", item.name, trade_quantity, item.rarity),
            true,
        )
        .field(
            "Wants (per lot)",
            format!("**{}** x{} ({:?})", wants.name, wants_amount, wants.rarity),
            true,
        )
        .field("Lots available", lots.to_string(), true)
        .color(serenity::Color::GOLD);

    let reply = ctx
        .send(
            CreateReply::default()
                .embed(confirm_embed)
                .components(vec![serenity::CreateActionRow::Buttons(vec![
                    serenity::CreateButton::new("confirm_new_trade")
                        .label("Post Trade")
                        .style(serenity::ButtonStyle::Success),
                    serenity::CreateButton::new("cancel_new_trade")
                        .label("Cancel")
                        .style(serenity::ButtonStyle::Danger),
                ])])
                .ephemeral(true),
        )
        .await?;

    let msg = reply.message().await?;
    let Some(component) = msg
        .await_component_interaction(ctx)
        .author_id(seller.id)
        .timeout(CONFIRM_TIMEOUT)
        .await
    else {
        reply
            .edit(
                ctx,
                CreateReply::default()
                    .content("⏰ Trade confirmation timed out.")
                    .components(vec![]),
            )
            .await
            .ok();
        return Ok(());
    };

    if component.data.custom_id == "cancel_new_trade" {
        component
            .create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::default()
                        .ephemeral(true)
                        .content("❌ Trade cancelled."),
                ),
            )
            .await?;
        return Ok(());
    }

    // Step 2: Post the trade using `lots` as the DB stock
    let trade = Trade::new(
        seller.id,
        *item,
        trade_quantity,
        *wants,
        wants_amount,
        lots,
    );
    let trade_id = ctx.data().trades.write(|db| db.insert(trade))?;
    ctx.data().trades.save()?;

    let embed = serenity::CreateEmbed::default()
        .title(format!("Trade by {}", seller.name))
        .thumbnail(avatar_url)
        .field(
            "Offering",
            format!(
                "**{}** x{} ({:?})",
                item.name, trade_quantity, item.rarity
            ),
            true,
        )
        .field(
            "Wants",
            format!(
                "**{}** x{} ({:?})",
                wants.name, wants_amount, wants.rarity
            ),
            true,
        )
        .field("Stock", format!("{lots} lot(s) remaining"), true)
        .color(serenity::Color::GOLD);

    let buttons = serenity::CreateActionRow::Buttons(vec![
        serenity::CreateButton::new(format!("buy_{trade_id}"))
            .label("Buy")
            .style(serenity::ButtonStyle::Success),
    ]);

    let data = ctx.data();
    let message = data
        .trade_posting_channel
        .send_message(
            ctx.http(),
            serenity::CreateMessage::default()
                .embed(embed)
                .components(vec![buttons]),
        )
        .await?;

    data.trades.write(|db| {
        if let Some(trade) = db.get_mut(trade_id) {
            trade.message_id = Some(message.id);
        }
    })?;
    data.trades.save()?;

    let http = ctx.serenity_context().http.clone();
    let channel_id = data.trade_posting_channel;
    tokio::spawn(async move {
        tokio::time::sleep(EXPIRATION_TIME).await;
        let _ = channel_id.delete_message(&http, message.id).await;
    });

    component
        .create_response(
            ctx,
            serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::default()
                    .ephemeral(true)
                    .content("✅ Trade posted!"),
            ),
        )
        .await?;

    Ok(())
}
