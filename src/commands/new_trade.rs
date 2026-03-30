use crate::commands::check_if_blacklisted;
use crate::database::Data;
use crate::database::supported_locale::{SupportedLocale, get_user_locale};
use crate::database::trade_db::{Trade, TradeKind, TradeStatus};
use crate::post::build_trade_embed;
use crate::print_err;
use crate::{Context, Res, item_name::ItemName, items::ITEMS, t};
use poise::serenity_prelude::{Message, UserId};
use poise::{CreateReply, serenity_prelude as serenity};
use std::time::Duration;

const CONFIRM_TIMEOUT: Duration = Duration::from_mins(1);

#[expect(clippy::unused_async)]
async fn autocomplete_item<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> impl Iterator<Item = serenity::AutocompleteChoice> + 'a {
    let locale = get_user_locale(ctx.data(), ctx.author().id);

    ITEMS.iter().filter_map(move |i| {
        let display = i.name.display(&locale);
        if display.to_lowercase().contains(&partial.to_lowercase())
            || i.name.to_str().to_lowercase().contains(&partial.to_lowercase())
        {
            Some(serenity::AutocompleteChoice::new(display, i.name.to_str()))
        } else {
            None
        }
    })
}

fn validate_input(
    trading_item: &str,
    for_item: &str,
    trade_quantity: u16,
    stock: u16,
    locale: &str,
) -> Res<(ItemName, ItemName, u16)> {
    let item = ItemName::from_str(trading_item).map_err(|_| {
        t!("error.invalid_trading_item", name = trading_item, locale = locale)
    })?;
    let wants = ItemName::from_str(for_item).map_err(|_| {
        t!("error.invalid_wanted_item", name = for_item, locale = locale)
    })?;

    if trade_quantity == 0 {
        return Err(t!("error.trade_quantity_zero", locale = locale).into());
    }

    let lots = stock / trade_quantity;
    if lots == 0 {
        return Err(t!(
            "error.stock_too_low",
            stock = stock,
            quantity = trade_quantity,
            locale = locale
        )
        .into());
    }

    Ok((item, wants, lots))
}

#[expect(clippy::too_many_arguments)]
fn build_confirm_embed(
    item: ItemName,
    wants: ItemName,
    item_rarity: &str,
    wants_rarity: &str,
    trade_quantity: u16,
    wants_amount: u16,
    lots: u16,
    avatar_url: String,
    locale: &str,
) -> serenity::CreateEmbed {
    serenity::CreateEmbed::default()
        .title(t!("new_trade.confirm.title", locale = locale))
        .description(t!(
            "new_trade.confirm.description",
            total = lots * trade_quantity,
            item = item.display(locale),
            lots = lots,
            quantity = trade_quantity,
            locale = locale
        ))
        .thumbnail(avatar_url)
        .field(
            t!("new_trade.confirm.field_offering", locale = locale),
            format!(
                "**{}** x{} ({})",
                item.display(locale),
                trade_quantity,
                item_rarity
            ),
            true,
        )
        .field(
            t!("new_trade.confirm.field_wants", locale = locale),
            format!(
                "**{}** x{} ({})",
                wants.display(locale),
                wants_amount,
                wants_rarity
            ),
            true,
        )
        .field(
            t!("new_trade.confirm.field_lots", locale = locale),
            lots.to_string(),
            true,
        )
        .color(serenity::Color::GOLD)
}

async fn show_confirmation(
    ctx: &Context<'_>,
    item: ItemName,
    wants: ItemName,
    trade_quantity: u16,
    wants_amount: u16,
    lots: u16,
    locale: &str,
) -> Res<Option<serenity::ComponentInteraction>> {
    let seller = ctx.author();
    let avatar_url =
        seller.avatar_url().unwrap_or_else(|| seller.default_avatar_url());

    let item_rarity = item.item().rarity.display(locale).into_owned();
    let wants_rarity = wants.item().rarity.display(locale).into_owned();

    let embed = build_confirm_embed(
        item,
        wants,
        &item_rarity,
        &wants_rarity,
        trade_quantity,
        wants_amount,
        lots,
        avatar_url,
        locale,
    );

    let reply = ctx
        .send(
            CreateReply::default()
                .embed(embed)
                .components(vec![serenity::CreateActionRow::Buttons(vec![
                    serenity::CreateButton::new("confirm_new_trade")
                        .label(t!(
                            "new_trade.confirm.button_post",
                            locale = locale
                        ))
                        .style(serenity::ButtonStyle::Success),
                    serenity::CreateButton::new("cancel_new_trade")
                        .label(t!(
                            "new_trade.confirm.button_cancel",
                            locale = locale
                        ))
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
                    .content(t!("new_trade.confirm.timed_out", locale = locale))
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
                        .content(t!(
                            "new_trade.confirm.cancelled",
                            locale = locale
                        )),
                ),
            )
            .await?;
        return Ok(None);
    }

    Ok(Some(component))
}

async fn send_post_embed(
    ctx: &Context<'_>,
    supported_locale: SupportedLocale,
    seller: &serenity::User,
    trade: &Trade,
    data: &Data,
    trade_id: u64,
) -> Res<Message> {
    let locale = supported_locale.to_locale();
    Ok(data
        .trade_posting_channel
        .get_channel(supported_locale)
        .send_message(
            ctx.http(),
            serenity::CreateMessage::default()
                .embed(build_trade_embed(trade, seller, locale))
                .components(vec![serenity::CreateActionRow::Buttons(vec![
                    serenity::CreateButton::new(format!("buy_{trade_id}"))
                        .label(t!("post.button_buy", locale = locale))
                        .style(serenity::ButtonStyle::Success),
                ])]),
        )
        .await
        .inspect_err(|e| {
            print_err(e);
            if let Err(rollback_err) =
                data.trades.write(|db| db.remove(trade_id))
            {
                print_err(&rollback_err);
            }
        })?)
}

#[expect(clippy::too_many_arguments)]
async fn post_trade(
    ctx: &Context<'_>,
    component: serenity::ComponentInteraction,
    item: ItemName,
    wants: ItemName,
    trade_quantity: u16,
    wants_amount: u16,
    lots: u16,
    locale: &str,
) -> Res<()> {
    let supported_locale = SupportedLocale::from_locale_fallback(locale);
    let seller = ctx.author();

    let item_obj = ITEMS.iter().find(|i| i.name == item).unwrap();
    let wants_obj = ITEMS.iter().find(|i| i.name == wants).unwrap();

    let trade = Trade::new(
        seller.id,
        *item_obj,
        trade_quantity,
        *wants_obj,
        wants_amount,
        lots,
        TradeKind::Normal,
        supported_locale,
    );

    let data = ctx.data();
    let trade_id = data.trades.write(|db| db.insert(trade.clone()))?;

    let english_message = send_post_embed(
        ctx,
        SupportedLocale::en_US,
        seller,
        &trade,
        data,
        trade_id,
    )
    .await?;

    let korean_message = match send_post_embed(
        ctx,
        SupportedLocale::ko_KR,
        seller,
        &trade,
        data,
        trade_id,
    )
    .await
    {
        Ok(m) => m,
        Err(e) => {
            english_message
                .delete(ctx.http())
                .await
                .inspect_err(print_err)
                .ok();
            return Err(e);
        }
    };

    data.trades.write(|db| {
        if let Some(t) = db.get_mut(trade_id) {
            t.english_message_id.insert(english_message.id);
            t.korean_message_id.insert(korean_message.id);
        }
    })?;
    data.trades.save()?;

    component
        .create_response(
            ctx,
            serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::default()
                    .ephemeral(true)
                    .content(t!("new_trade.posted", locale = locale)),
            ),
        )
        .await?;

    Ok(())
}

fn check_dupe(
    data: &Data,
    seller: UserId,
    wants: ItemName,
    wants_amount: u16,
    item: ItemName,
    item_quantity: u16,
    lots: u16,
) -> Res<Option<Trade>> {
    let test_trade = Trade::new(
        seller,
        *item.item(),
        item_quantity,
        *wants.item(),
        wants_amount,
        lots,
        TradeKind::Normal,
        SupportedLocale::default(),
    );

    Ok(data.trades.read(|db| {
        db.iter().find_map(|t| {
            if matches!(t.1.status(), TradeStatus::Running)
                && *t.1 == test_trade
            {
                Some(t.1.clone())
            } else {
                None
            }
        })
    })?)
}

/// Make a new trade request
/// 새로운 거래 요청을 만듭니다
#[poise::command(slash_command, interaction_context = "Guild")]
pub async fn new_trade(
    ctx: Context<'_>,

    #[autocomplete = "autocomplete_item"]
    #[description = "The item you are offering"]
    #[description_localized("ko", "제공할 아이템")]
    trading_item: String,

    #[description = "How many of the item you are offering per lot"]
    #[description_localized("ko", "개당 당 제시할 아이템 수량")]
    trade_quantity: u16,

    #[autocomplete = "autocomplete_item"]
    #[description = "The item you want in return"]
    #[description_localized("ko", "받고 싶은 아이템")]
    for_item: String,

    #[description = "How many of the wanted item you expect per lot"]
    #[description_localized("ko", "원하는 아이템의 예상하는 갯수")]
    wants_amount: u16,

    #[description = "Total amount of the offered item you have in stock"]
    #[description_localized("ko", "보유 중인 총 재고량")]
    stock: u16,
) -> Res<()> {
    let locale = get_user_locale(ctx.data(), ctx.author().id);
    check_if_blacklisted(ctx, &locale).await?;

    let (item, wants, lots) = validate_input(
        &trading_item,
        &for_item,
        trade_quantity,
        stock,
        &locale,
    )?;

    if let Some(trade) = check_dupe(
        ctx.data(),
        ctx.author().id,
        wants,
        wants_amount,
        item,
        trade_quantity,
        lots,
    )? {
        let channel_locale =
            SupportedLocale::from_locale_fallback(&locale).korean_or_english();
        // https://discord.com/channels/1486558411008114788/1486735432816263189/1486736920066396222
        let guild_id = ctx
            .guild_id()
            .ok_or("Unable to get Guild ID, this shouldn't ever happen")?;
        let channel_id =
            ctx.data().trade_posting_channel.get_channel(channel_locale);

        let message_id = match channel_locale {
            SupportedLocale::ko_KR => trade.korean_message_id,
            _ => trade.english_message_id,
        }
        .id()?;

        let message_link = format!(
            "https://discord.com/channels/{guild_id}/{channel_id}/{message_id}"
        );

        return Err(t!(
            "error.duplicate_post",
            locale = locale,
            message_link = message_link
        )
        .into());
    }

    let Some(component) = show_confirmation(
        &ctx,
        item,
        wants,
        trade_quantity,
        wants_amount,
        lots,
        &locale,
    )
    .await?
    else {
        return Ok(());
    };
    post_trade(
        &ctx,
        component,
        item,
        wants,
        trade_quantity,
        wants_amount,
        lots,
        &locale,
    )
    .await
}
