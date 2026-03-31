use poise::serenity_prelude::{self as serenity, CacheHttp};

use crate::{
    Res,
    database::{
        Data,
        auction_db::RunningAuction,
        supported_locale::SupportedLocale,
        trade_db::{Trade, TradeKind},
    },
    magic_numbers::TRADE_EXPIRATION_TIME,
};

// ── Trade embeds ──────────────────────────────────────────────────────────────

/// Builds a trade embed for the given trade and seller.
/// Handles Normal vs Auction layout, sold out state, footer, and expiry timestamp.
pub fn build_trade_embed(
    trade: &Trade,
    seller: &serenity::User,
    post_locale: &str,
) -> serenity::CreateEmbed {
    let avatar_url =
        seller.avatar_url().unwrap_or_else(|| seller.default_avatar_url());
    let sold_out = trade.is_sold_out();

    let footer = serenity::CreateEmbedFooter::new(t!(
        "post.footer_buyers",
        locale = post_locale,
        count = trade.buyers.len()
    ));

    match trade.kind {
        TradeKind::Normal => build_normal_embed(
            trade,
            seller,
            sold_out,
            avatar_url,
            footer,
            post_locale,
        ),
        TradeKind::Auction => build_completed_auction_embed(
            trade,
            seller,
            avatar_url,
            footer,
            post_locale,
        ),
    }
}

fn build_normal_embed(
    trade: &Trade,
    seller: &serenity::User,
    sold_out: bool,
    avatar_url: String,
    footer: serenity::CreateEmbedFooter,
    post_locale: &str,
) -> serenity::CreateEmbed {
    let title = if sold_out {
        t!("post.title_sold_out", locale = post_locale, seller = seller.name)
    } else {
        t!("post.title", locale = post_locale, seller = seller.name)
    };

    let expires_unix = trade
        .created_at()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + TRADE_EXPIRATION_TIME.as_secs();

    serenity::CreateEmbed::default()
        .title(title)
        .description(t!(
            "post.expires",
            locale = post_locale,
            unix = expires_unix
        ))
        .thumbnail(avatar_url)
        .field(
            t!("post.field_offering", locale = post_locale),
            format!(
                "**{}** x{} ({})",
                trade.item.name.display(post_locale),
                trade.quantity,
                trade.item.rarity.display(post_locale)
            ),
            true,
        )
        .field(
            t!("post.field_wants", locale = post_locale),
            format!(
                "**{}** x{} ({})",
                trade.wants.name.display(post_locale),
                trade.wanted_amount,
                trade.wants.rarity.display(post_locale)
            ),
            true,
        )
        .field(
            t!("post.field_stock", locale = post_locale),
            if sold_out {
                t!("post.stock_sold_out", locale = post_locale)
            } else {
                t!("post.stock_value", locale = post_locale, lots = trade.stock)
            },
            true,
        )
        .footer(footer)
        .color(if sold_out {
            serenity::Color::DARK_GREY
        } else {
            serenity::Color::GOLD
        })
}

/// Embed for a completed auction stored as a `Trade` in the trades database.
fn build_completed_auction_embed(
    trade: &Trade,
    seller: &serenity::User,
    avatar_url: String,
    footer: serenity::CreateEmbedFooter,
    post_locale: &str,
) -> serenity::CreateEmbed {
    let sold_out = trade.is_sold_out();

    let title = if sold_out {
        t!(
            "auction.post.title_ended",
            locale = post_locale,
            seller = seller.name
        )
    } else {
        t!("auction.post.title", locale = post_locale, seller = seller.name)
    };

    let expires_unix = trade
        .created_at()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + TRADE_EXPIRATION_TIME.as_secs();

    serenity::CreateEmbed::default()
        .title(title)
        .description(t!(
            "auction.post.ended_at",
            locale = post_locale,
            unix = expires_unix
        ))
        .thumbnail(avatar_url)
        .field(
            t!("auction.post.field_item", locale = post_locale),
            format!(
                "**{}** x{} ({})",
                trade.item.name.display(post_locale),
                trade.quantity,
                trade.item.rarity.display(post_locale)
            ),
            true,
        )
        .field(
            t!("auction.post.field_min_price", locale = post_locale),
            format!(
                "**{}** x{} ({})",
                trade.wants.name.display(post_locale),
                trade.wanted_amount,
                trade.wants.rarity.display(post_locale)
            ),
            true,
        )
        .footer(footer)
        .color(serenity::Color::DARK_GREY)
}

/// Fetches the trade, builds the embed, and edits the post message.
/// Should be called after any stock change.
pub async fn update_post(
    ctx: &serenity::Context,
    data: &Data,
    trade_id: u64,
    locale: SupportedLocale,
) -> Res<()> {
    let post_locale = locale.korean_or_english().to_locale();
    let trade = {
        let db = data.trades.borrow_data()?;
        match db.get(trade_id) {
            Some(t) => t.clone(),
            None => return Ok(()),
        }
    };

    let message_info = match locale {
        SupportedLocale::en_US => trade.english_message_id,
        SupportedLocale::ko_KR => trade.korean_message_id,
    };

    if message_info.deleted {
        return Ok(());
    }

    let message_id = message_info.id()?;
    let seller = trade.seller.to_user(ctx).await?;
    let sold_out = trade.is_sold_out();
    let embed = build_trade_embed(&trade, &seller, post_locale);

    data.trade_posting_channel
        .get_channel(locale)
        .edit_message(
            ctx.http(),
            message_id,
            serenity::EditMessage::default().embed(embed).components(vec![
                serenity::CreateActionRow::Buttons(vec![
                    serenity::CreateButton::new(format!("buy_{trade_id}"))
                        .label(t!("post.button_buy", locale = post_locale))
                        .style(serenity::ButtonStyle::Success)
                        .disabled(sold_out),
                ]),
            ]),
        )
        .await?;

    Ok(())
}

// ── Running auction embeds ────────────────────────────────────────────────────

/// Builds an embed for a currently running auction.
/// `highest_bidder_name` should be pre-resolved by the caller.
pub fn build_auction_embed(
    auction: &RunningAuction,
    seller: &serenity::User,
    highest_bidder_name: Option<&str>,
    post_locale: &str,
) -> serenity::CreateEmbed {
    let avatar_url =
        seller.avatar_url().unwrap_or_else(|| seller.default_avatar_url());

    let expired = auction.is_expired();

    let end_unix = auction
        .end_time
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let title = if expired {
        t!(
            "auction.post.title_ended",
            locale = post_locale,
            seller = seller.name
        )
    } else {
        t!("auction.post.title", locale = post_locale, seller = seller.name)
    };

    let current_bid_value = if let Some((_, amount)) = auction.highest_bid() {
        if let Some(name) = highest_bidder_name {
            t!(
                "auction.post.current_bid",
                locale = post_locale,
                amount = amount,
                currency = auction.currency_item.name.display(post_locale),
                bidder = name
            )
        } else {
            t!(
                "auction.post.current_bid_unknown",
                locale = post_locale,
                amount = amount,
                currency = auction.currency_item.name.display(post_locale)
            )
        }
    } else {
        t!("auction.post.no_bids", locale = post_locale)
    };

    let footer = serenity::CreateEmbedFooter::new(t!(
        "post.footer_bidders",
        locale = post_locale,
        count = auction.bids.len()
    ));

    serenity::CreateEmbed::default()
        .title(title)
        .thumbnail(avatar_url)
        .description(if expired {
            t!("auction.post.ended_at", locale = post_locale, unix = end_unix)
        } else {
            t!("auction.post.ends_at", locale = post_locale, unix = end_unix)
        })
        .field(
            t!("auction.post.field_item", locale = post_locale),
            format!(
                "**{}** x{} ({})",
                auction.item.name.display(post_locale),
                auction.quantity,
                auction.item.rarity.display(post_locale)
            ),
            true,
        )
        .field(
            t!("auction.post.field_min_price", locale = post_locale),
            format!(
                "**{}** x{} ({})",
                auction.currency_item.name.display(post_locale),
                auction.min_price,
                auction.currency_item.rarity.display(post_locale)
            ),
            true,
        )
        .field(
            t!("auction.post.field_current_bid", locale = post_locale),
            current_bid_value,
            false,
        )
        .footer(footer)
        .color(if expired {
            serenity::Color::DARK_GREY
        } else {
            serenity::Color::DARK_PURPLE
        })
}

/// Fetches the running auction, resolves the highest bidder's name,
/// builds the embed, and edits both channel posts.
pub async fn update_auction_post(
    ctx: &serenity::Context,
    data: &Data,
    auction_id: u64,
    locale: SupportedLocale,
) -> Res<()> {
    let post_locale = locale.korean_or_english().to_locale();
    let auction = {
        let db = data.running_auctions.borrow_data()?;
        match db.get(auction_id) {
            Some(a) => a.clone(),
            None => return Ok(()),
        }
    };

    let message_info = match locale {
        SupportedLocale::en_US => auction.english_message_id,
        SupportedLocale::ko_KR => auction.korean_message_id,
    };

    if message_info.deleted {
        return Ok(());
    }

    let message_id = message_info.id()?;
    let seller = auction.seller.to_user(ctx).await?;

    let highest_bidder_name =
        if let Some((bidder_id, _)) = auction.highest_bid() {
            bidder_id.to_user(ctx).await.ok().map(|u| u.name)
        } else {
            None
        };

    let expired = auction.is_expired();
    let embed = build_auction_embed(
        &auction,
        &seller,
        highest_bidder_name.as_deref(),
        post_locale,
    );

    data.trade_posting_channel
        .get_channel(locale)
        .edit_message(
            ctx.http(),
            message_id,
            serenity::EditMessage::default().embed(embed).components(
                if expired {
                    vec![]
                } else {
                    vec![serenity::CreateActionRow::Buttons(vec![
                        serenity::CreateButton::new(format!(
                            "bid_{auction_id}"
                        ))
                        .label(t!(
                            "auction.post.button_bid",
                            locale = post_locale
                        ))
                        .style(serenity::ButtonStyle::Primary),
                    ])]
                },
            ),
        )
        .await?;

    Ok(())
}
