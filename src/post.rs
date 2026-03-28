use poise::serenity_prelude::{self as serenity, CacheHttp};

use crate::{
    Res,
    database::{
        Data,
        trade_db::{Trade, TradeKind},
    },
    magic_numbers::TRADE_EXPIRATION_TIME,
};

/// Builds a trade embed for the given trade and seller.
/// Handles Normal vs Auction layout, sold out state, footer, and expiry timestamp.
pub fn build_trade_embed(
    trade: &Trade,
    seller: &serenity::User,
) -> serenity::CreateEmbed {
    let avatar_url =
        seller.avatar_url().unwrap_or_else(|| seller.default_avatar_url());
    let sold_out = trade.is_sold_out();

    let footer = serenity::CreateEmbedFooter::new(format!(
        "{} buyer(s)",
        trade.buyers.len()
    ));

    match trade.kind {
        TradeKind::Normal => {
            build_normal_embed(trade, seller, sold_out, avatar_url, footer)
        }
        TradeKind::Auction => {
            build_auction_embed(trade, seller, sold_out, &avatar_url, &footer)
        }
    }
}

fn build_normal_embed(
    trade: &Trade,
    seller: &serenity::User,
    sold_out: bool,
    avatar_url: String,
    footer: serenity::CreateEmbedFooter,
) -> serenity::CreateEmbed {
    let title = if sold_out {
        format!("[SOLD OUT] Trade by {}", seller.name)
    } else {
        format!("Trade by {}", seller.name)
    };

    let expires_unix = trade
        .created_at()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + TRADE_EXPIRATION_TIME.as_secs();

    serenity::CreateEmbed::default()
        .title(title)
        .description(format!("Expires <t:{expires_unix}:R>"))
        .thumbnail(avatar_url)
        .field(
            "Offering",
            format!(
                "**{}** x{} ({})",
                trade.item, trade.quantity, trade.item.rarity
            ),
            true,
        )
        .field(
            "Wants",
            format!(
                "**{}** x{} ({})",
                trade.wants, trade.wanted_amount, trade.wants.rarity
            ),
            true,
        )
        .field(
            "Stock",
            if sold_out {
                "Sold out".to_string()
            } else {
                format!("{} lot(s) remaining", trade.stock)
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

#[expect(unused, reason = "TODO")]
fn build_auction_embed(
    trade: &Trade,
    seller: &serenity::User,
    sold_out: bool,
    avatar_url: &str,
    footer: &serenity::CreateEmbedFooter,
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
) -> Res<()> {
    let (message_id, trade) = {
        let db = data.trades.borrow_data()?;
        match db.get(trade_id) {
            Some(t) => match t.message_id {
                Some(mid) => (mid, t.clone()),
                None => return Ok(()),
            },
            None => return Ok(()),
        }
    };

    if trade.message_deleted {
        return Ok(());
    }

    let seller = trade.seller.to_user(ctx).await?;
    let sold_out = trade.is_sold_out();
    let embed = build_trade_embed(&trade, &seller);

    data.trade_posting_channel
        .edit_message(
            ctx.http(),
            message_id,
            serenity::EditMessage::default().embed(embed).components(vec![
                serenity::CreateActionRow::Buttons(vec![
                    serenity::CreateButton::new(format!("buy_{trade_id}"))
                        .label("Buy")
                        .style(serenity::ButtonStyle::Success)
                        .disabled(sold_out),
                ]),
            ]),
        )
        .await?;

    Ok(())
}
