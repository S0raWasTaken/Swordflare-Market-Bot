use poise::serenity_prelude::{self as serenity, CacheHttp};

use crate::{
    Res,
    database::{
        Data,
        supported_locale::SupportedLocale,
        trade_db::{Trade, TradeKind},
    },
    magic_numbers::TRADE_EXPIRATION_TIME,
};

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
        TradeKind::Auction => build_auction_embed(
            trade,
            seller,
            sold_out,
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
        // format!("[SOLD OUT] Trade by {}", seller.name)
        t!("post.title_sold_out", locale = post_locale, seller = seller.name)
    } else {
        // format!("Trade by {}", seller.name)
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

#[expect(unused, clippy::needless_pass_by_value, reason = "TODO")]
fn build_auction_embed(
    trade: &Trade,
    seller: &serenity::User,
    sold_out: bool,
    avatar_url: String,
    footer: serenity::CreateEmbedFooter,
    post_locale: &str,
) -> serenity::CreateEmbed {
    // TODO: Auction layout
    todo!()
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
