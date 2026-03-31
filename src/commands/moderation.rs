use poise::serenity_prelude::{Message, MessageId, User};

use crate::{
    Context, Res,
    cleanup::clean_database,
    commands::is_bot_admin,
    database::{
        auction_db::{AuctionData, RunningAuction},
        trade_db::TradeData,
    },
};

const POST_NOT_FOUND: &str = "❌ Couldn't find post in database. You may as well delete this message manually.";

fn try_set_in_trades_db(
    db: &mut TradeData,
    msg_id: MessageId,
) -> Res<Option<RunningAuction>> {
    let Some((trade_id, _)) = db.iter().find(|e| {
        e.1.english_message_id.is_eq(msg_id)
            || e.1.korean_message_id.is_eq(msg_id)
    }) else {
        return Err("This message will be shadowed".into());
    };

    db.get_mut(trade_id).expect("Write lock????? Hello????").moderated = true;
    Ok(None)
}

fn try_remove_from_auctions_db(
    db: &mut AuctionData,
    msg_id: MessageId,
) -> Res<Option<RunningAuction>> {
    let Some((auction_id, auction)) = db.iter().find(|e| {
        e.1.english_message_id.is_eq(msg_id)
            || e.1.korean_message_id.is_eq(msg_id)
    }) else {
        return Err(POST_NOT_FOUND.into());
    };
    let auction = auction.clone();

    db.remove(auction_id);
    Ok(Some(auction))
}

#[poise::command(
    context_menu_command = "Mark post as invalid",
    check = "is_bot_admin",
    interaction_context = "Guild"
)]
pub async fn mark_as_invalid(ctx: Context<'_>, msg: Message) -> Res<()> {
    ctx.defer_ephemeral().await?;

    let trades_result =
        ctx.data().trades.write(|db| try_set_in_trades_db(db, msg.id))?;

    if trades_result.is_ok() {
        ctx.data().trades.save()?;
    } else {
        let auction = ctx
            .data()
            .running_auctions
            .write(|db| try_remove_from_auctions_db(db, msg.id))??;

        if let Some(auction) = auction {
            auction.delete_messages(ctx).await?;
        }
        ctx.data().running_auctions.save()?;
    }

    clean_database(ctx.serenity_context(), ctx.data()).await?;

    ctx.say("✅ Trade marked as invalid.").await?;
    Ok(())
}

#[poise::command(
    context_menu_command = "Blacklist User",
    check = "is_bot_admin",
    interaction_context = "Guild"
)]
pub async fn blacklist_user(ctx: Context<'_>, user: User) -> Res<()> {
    ctx.defer().await?;
    let inserted = ctx.data().blacklist.write(|db| db.insert(user.id))?;

    if !inserted {
        return Err("❌ User is already blacklisted".into());
    }

    ctx.data().blacklist.save()?;

    ctx.say("✅ User added to the blacklist").await?;
    Ok(())
}

#[poise::command(
    context_menu_command = "Unblacklist User",
    check = "is_bot_admin",
    interaction_context = "Guild"
)]
pub async fn unblacklist_user(ctx: Context<'_>, user: User) -> Res<()> {
    ctx.defer().await?;
    let removed = ctx.data().blacklist.write(|db| db.remove(&user.id))?;

    if !removed {
        return Err("❌ User is not blacklisted".into());
    }

    ctx.data().blacklist.save()?;

    ctx.say("✅ User removed from the blacklist").await?;
    Ok(())
}
