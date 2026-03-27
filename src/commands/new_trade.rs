use crate::database::trade_db::{Trade, TradeKind};
use crate::post::build_trade_embed;
use crate::print_err;
use crate::{
    Context, Res,
    items::{ITEMS, Item},
};
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

fn validate_input<'a>(
    trading_item: &str,
    for_item: &str,
    trade_quantity: u16,
    stock: u16,
) -> Res<(&'a Item, &'a Item, u16)> {
    let item = ITEMS
        .iter()
        .find(|i| i.name.to_lowercase() == trading_item.to_lowercase())
        .ok_or_else(|| format!("Invalid trading item: '{trading_item}'"))?;
    let wants = ITEMS
        .iter()
        .find(|i| i.name.to_lowercase() == for_item.to_lowercase())
        .ok_or_else(|| format!("Invalid wanted item: '{for_item}'"))?;

    if trade_quantity == 0 {
        return Err("Trade quantity must be greater than zero.".into());
    }

    let lots = stock / trade_quantity;
    if lots == 0 {
        return Err(format!(
            "Stock ({stock}) must be at least equal to trade quantity ({trade_quantity})."
        )
        .into());
    }

    Ok((item, wants, lots))
}

fn build_confirm_embed(
    item: &Item,
    wants: &Item,
    trade_quantity: u16,
    wants_amount: u16,
    lots: u16,
    avatar_url: String,
) -> serenity::CreateEmbed {
    serenity::CreateEmbed::default()
        .title("Confirm Trade Post")
        .description(format!(
            "You're about to sell a total of **x{} {}** across **{lots}** lot(s) of x{trade_quantity} each.",
            lots * trade_quantity,
            item,
        ))
        .thumbnail(avatar_url)
        .field("Offering (per lot)", format!("**{}** x{} ({})", item, trade_quantity, item.rarity), true)
        .field("Wants (per lot)", format!("**{}** x{} ({})", wants, wants_amount, wants.rarity), true)
        .field("Lots available", lots.to_string(), true)
        .color(serenity::Color::GOLD)
}

async fn show_confirmation(
    ctx: &Context<'_>,
    item: &Item,
    wants: &Item,
    trade_quantity: u16,
    wants_amount: u16,
    lots: u16,
) -> Res<Option<serenity::ComponentInteraction>> {
    let seller = ctx.author();
    let avatar_url =
        seller.avatar_url().unwrap_or_else(|| seller.default_avatar_url());
    let embed = build_confirm_embed(
        item,
        wants,
        trade_quantity,
        wants_amount,
        lots,
        avatar_url,
    );

    let reply = ctx
        .send(
            CreateReply::default()
                .embed(embed)
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
                *ctx,
                CreateReply::default()
                    .content("⏰ Trade confirmation timed out.")
                    .components(vec![]),
            )
            .await
            .ok();
        return Ok(None);
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
        return Ok(None);
    }

    Ok(Some(component))
}

async fn post_trade(
    ctx: &Context<'_>,
    component: serenity::ComponentInteraction,
    item: &Item,
    wants: &Item,
    trade_quantity: u16,
    wants_amount: u16,
    lots: u16,
) -> Res<()> {
    let seller = ctx.author();
    let trade = Trade::new(
        seller.id,
        *item,
        trade_quantity,
        *wants,
        wants_amount,
        lots,
        TradeKind::Normal,
    );

    let data = ctx.data();
    let trade_id = data.trades.write(|db| db.insert(trade.clone()))?;

    let message = data
        .trade_posting_channel
        .send_message(
            ctx.http(),
            serenity::CreateMessage::default()
                .embed(build_trade_embed(&trade, seller))
                .components(vec![serenity::CreateActionRow::Buttons(vec![
                    serenity::CreateButton::new(format!("buy_{trade_id}"))
                        .label("Buy")
                        .style(serenity::ButtonStyle::Success),
                ])]),
        )
        .await
        .inspect_err(|e| {
            // Rollback the in-memory insert on failure
            print_err(e);
            if let Err(rollback_err) =
                data.trades.write(|db| db.remove(trade_id))
            {
                print_err(&rollback_err);
            }
        })?;

    data.trades.write(|db| {
        if let Some(t) = db.get_mut(trade_id) {
            t.message_id = Some(message.id);
        }
    })?;
    data.trades.save()?;

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

#[poise::command(slash_command)]
pub async fn new_trade(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_item"]
    #[description = "The item you are offering"]
    trading_item: String,
    #[description = "How many of the item you are offering per lot"]
    trade_quantity: u16,
    #[autocomplete = "autocomplete_item"]
    #[description = "The item you want in return"]
    for_item: String,
    #[description = "How many of the wanted item you expect per lot"]
    wants_amount: u16,
    #[description = "Total amount of the offered item you have in stock"] stock: u16,
) -> Res<()> {
    let (item, wants, lots) =
        validate_input(&trading_item, &for_item, trade_quantity, stock)?;
    let Some(component) = show_confirmation(
        &ctx,
        item,
        wants,
        trade_quantity,
        wants_amount,
        lots,
    )
    .await?
    else {
        return Ok(());
    };
    post_trade(&ctx, component, item, wants, trade_quantity, wants_amount, lots)
        .await
}
