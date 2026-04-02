use poise::{
    CreateReply,
    serenity_prelude::{
        self as serenity, ButtonStyle, CreateActionRow, CreateButton,
    },
};
use std::time::Duration;

use crate::{
    Context, Res,
    cleanup::resolve_auction,
    commands::{check_if_blacklisted, check_if_paused},
    database::{
        Data,
        auction_db::RunningAuction,
        supported_locale::{SupportedLocale, get_user_locale},
    },
    duration::parse_duration,
    item_name::ItemName,
    items::ITEMS,
    magic_numbers::TRADE_EXPIRATION_TIME,
    post::build_auction_embed,
    print_err, t,
};

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

const MIN_AUCTION_DURATION: Duration = Duration::from_mins(1);

fn validate_input(
    item: &str,
    quantity: u64,
    currency_item: &str,
    min_price: u64,
    duration_str: &str,
    locale: &str,
) -> Res<(ItemName, ItemName, Duration)> {
    let item = ItemName::from_str(item).map_err(|_| {
        t!("error.invalid_trading_item", name = item, locale = locale)
    })?;
    let currency = ItemName::from_str(currency_item).map_err(|_| {
        t!("error.invalid_wanted_item", name = currency_item, locale = locale)
    })?;

    if quantity == 0 {
        return Err(t!("error.trade_quantity_zero", locale = locale).into());
    }

    if min_price == 0 {
        return Err(t!("auction.error.min_price_zero", locale = locale).into());
    }

    let duration = parse_duration(duration_str).map_err(|e| format!("{e}"))?;

    if duration < MIN_AUCTION_DURATION {
        return Err(t!(
            "auction.error.duration_too_short",
            min = MIN_AUCTION_DURATION.as_secs() / 60,
            locale = locale
        )
        .into());
    }

    if duration > TRADE_EXPIRATION_TIME {
        return Err(t!(
            "auction.error.duration_too_long",
            max = TRADE_EXPIRATION_TIME.as_secs() / 3600,
            locale = locale
        )
        .into());
    }

    Ok((item, currency, duration))
}

#[expect(clippy::too_many_lines, reason = "Come on, 101/100")]
async fn show_confirmation(
    ctx: Context<'_>,
    item: ItemName,
    quantity: u64,
    currency: ItemName,
    min_price: u64,
    duration: Duration,
    locale: &str,
) -> Res<Option<serenity::ComponentInteraction>> {
    let seller = ctx.author();
    let avatar_url =
        seller.avatar_url().unwrap_or_else(|| seller.default_avatar_url());

    let hours = duration.as_secs() / 3600;
    let mins = (duration.as_secs() % 3600) / 60;
    let duration_display = match (hours, mins) {
        (h, 0) => format!("{h}h"),
        (0, m) => format!("{m}m"),
        (h, m) => format!("{h}h{m}m"),
    };

    let embed = serenity::CreateEmbed::default()
        .title(t!("auction.confirm.title", locale = locale))
        .thumbnail(avatar_url)
        .field(
            t!("auction.confirm.field_item", locale = locale),
            format!(
                "**{}** x{} ({})",
                item.display(locale),
                quantity,
                item.item().rarity.display(locale)
            ),
            true,
        )
        .field(
            t!("auction.confirm.field_currency", locale = locale),
            format!(
                "**{}** ({})",
                currency.display(locale),
                currency.item().rarity.display(locale)
            ),
            true,
        )
        .field(
            t!("auction.confirm.field_min_price", locale = locale),
            min_price.to_string(),
            true,
        )
        .field(
            t!("auction.confirm.field_duration", locale = locale),
            duration_display,
            true,
        )
        .color(serenity::Color::DARK_PURPLE);

    let reply = ctx
        .send(
            CreateReply::default()
                .embed(embed)
                .components(vec![serenity::CreateActionRow::Buttons(vec![
                    serenity::CreateButton::new("confirm_new_auction")
                        .label(t!(
                            "new_trade.confirm.button_post",
                            locale = locale
                        ))
                        .style(serenity::ButtonStyle::Success),
                    serenity::CreateButton::new("cancel_new_auction")
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
                ctx,
                CreateReply::default()
                    .content(t!("new_trade.confirm.timed_out", locale = locale))
                    .components(vec![]),
            )
            .await
            .ok();
        return Ok(None);
    };

    if component.data.custom_id == "cancel_new_auction" {
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

pub fn auction_buttons(auction_id: u64, locale: &str) -> CreateActionRow {
    CreateActionRow::Buttons(vec![
        CreateButton::new(format!("bid_{auction_id}"))
            .label(t!("auction.post.button_bid", locale = locale))
            .style(ButtonStyle::Primary),
        CreateButton::new(format!("au_cancel_{auction_id}"))
            .label(t!("buy.confirm.button_cancel", locale = locale))
            .style(ButtonStyle::Danger),
    ])
}

async fn send_auction_embed(
    ctx: Context<'_>,
    supported_locale: SupportedLocale,
    seller: &serenity::User,
    auction: &RunningAuction,
    data: &Data,
    auction_id: u64,
) -> Res<serenity::Message> {
    let locale = supported_locale.to_locale();
    Ok(data
        .auctions_channel
        .get_channel(supported_locale)
        .send_message(
            ctx.http(),
            serenity::CreateMessage::default()
                .embed(build_auction_embed(auction, seller, None, locale))
                .components(vec![auction_buttons(auction_id, locale)]),
        )
        .await
        .inspect_err(|e| {
            print_err(e);
            if let Err(rollback_err) =
                data.running_auctions.write(|db| db.remove(auction_id))
            {
                print_err(&rollback_err);
            }
        })?)
}

#[expect(clippy::too_many_arguments)]
async fn post_auction(
    ctx: Context<'_>,
    component: serenity::ComponentInteraction,
    item: ItemName,
    quantity: u64,
    currency: ItemName,
    min_price: u64,
    duration: Duration,
    locale: &str,
) -> Res<RunningAuction> {
    let supported_locale = SupportedLocale::from_locale_fallback(locale);
    let seller = ctx.author();

    let item_obj = item.item();
    let currency_obj = currency.item();

    let mut auction = RunningAuction::new(
        seller.id,
        *item_obj,
        quantity,
        *currency_obj,
        min_price,
        duration,
        supported_locale,
    );

    let data = ctx.data();
    let auction_id =
        data.running_auctions.write(|db| db.insert(auction.clone()))?;

    let english_message = send_auction_embed(
        ctx,
        SupportedLocale::en_US,
        seller,
        &auction,
        data,
        auction_id,
    )
    .await?;

    let korean_message = match send_auction_embed(
        ctx,
        SupportedLocale::ko_KR,
        seller,
        &auction,
        data,
        auction_id,
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
            // Also remove the auction from the database
            if let Err(rollback_err) =
                data.running_auctions.write(|db| db.remove(auction_id))
            {
                print_err(&rollback_err);
            }
            return Err(e);
        }
    };

    auction.english_message_id.insert(english_message.id);
    auction.korean_message_id.insert(korean_message.id);

    data.running_auctions.write(|db| {
        if let Some(a) = db.get_mut(auction_id) {
            a.english_message_id.insert(english_message.id);
            a.korean_message_id.insert(korean_message.id);
        }
    })?;

    data.running_auctions.save()?;

    // Spawn a task to resolve the auction when it ends
    let ctx_clone = ctx.serenity_context().clone();
    let data_clone = data.clone();
    let end_time = auction.end_time;
    tokio::spawn(async move {
        if let Ok(remaining) =
            end_time.duration_since(std::time::SystemTime::now())
        {
            tokio::time::sleep(remaining).await;
        }
        // Re-fetch from DB to get all bids placed since creation
        let auction = match data_clone.running_auctions.borrow_data() {
            Ok(db) => match db.get(auction_id) {
                Some(a) => a.clone(),
                None => return, // already resolved by cleanup
            },
            Err(e) => {
                print_err(&e);
                return;
            }
        };
        resolve_auction(&ctx_clone, &data_clone, auction_id, auction)
            .await
            .inspect_err(print_err)
            .ok();
    });

    component
        .create_response(
            ctx,
            serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::default()
                    .ephemeral(true)
                    .content(t!("auction.posted", locale = locale)),
            ),
        )
        .await?;

    Ok(auction)
}

/// Start a new auction
/// 새로운 경매를 시작합니다
#[poise::command(slash_command, interaction_context = "Guild")]
pub async fn new_auction(
    ctx: Context<'_>,

    #[autocomplete = "autocomplete_item"]
    #[description = "The item you are auctioning"]
    #[description_localized("ko", "경매할 아이템")]
    item: String,

    #[description = "How many of the item you are auctioning"]
    #[description_localized("ko", "경매할 아이템 수량")]
    quantity: u64,

    #[autocomplete = "autocomplete_item"]
    #[description = "The item used as currency for bids"]
    #[description_localized("ko", "입찰에 사용할 아이템")]
    currency_item: String,

    #[description = "Minimum starting bid"]
    #[description_localized("ko", "최소 시작 입찰가")]
    min_price: u64,

    #[description = "Duration e.g. 1h30m (max 48h)"]
    #[description_localized("ko", "경매 기간 예: 1h30m (최대 48h)")]
    duration: String,
) -> Res<()> {
    let locale = &get_user_locale(ctx.data(), ctx.author().id);
    check_if_blacklisted(ctx, locale).await?;
    check_if_paused(ctx, locale)?;

    let (item, currency, duration) = validate_input(
        &item,
        quantity,
        &currency_item,
        min_price,
        &duration,
        locale,
    )?;

    let Some(component) = show_confirmation(
        ctx, item, quantity, currency, min_price, duration, locale,
    )
    .await?
    else {
        return Ok(());
    };

    let auction = post_auction(
        ctx, component, item, quantity, currency, min_price, duration, locale,
    )
    .await?;

    ctx.data()
        .log(ctx.http(), &auction.display_log(ctx.data())?)
        .await
        .inspect_err(print_err)
        .ok();

    Ok(())
}
