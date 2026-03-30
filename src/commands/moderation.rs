use poise::serenity_prelude::{Message, User};

use crate::{
    Context, Error, Res, cleanup::clean_database, commands::is_bot_admin,
};

#[poise::command(
    context_menu_command = "Mark post as invalid",
    check = "is_bot_admin",
    interaction_context = "Guild"
)]
pub async fn mark_as_invalid(ctx: Context<'_>, msg: Message) -> Res<()> {
    ctx.defer_ephemeral().await?;
    ctx.data().trades.write(|db| {
        let Some(trade_id) = db.iter().find(|e| {
            e.1.english_message_id.is_eq(msg.id)
                || e.1.korean_message_id.is_eq(msg.id)
        }).map(|e| e.0) else {
            return Err::<(), Error>(
                "❌ Couldn't find post in database. You may as well delete this message manually.".into()
            );
        };

        db.get_mut(trade_id).expect("Write lock????? Hello????").moderated = true;
        Ok(())
    })??;

    ctx.data().trades.save()?;
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
    ctx.defer_ephemeral().await?;
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
    ctx.defer_ephemeral().await?;
    let removed = ctx.data().blacklist.write(|db| db.remove(&user.id))?;

    if !removed {
        return Err("❌ User is not blacklisted".into());
    }

    ctx.data().blacklist.save()?;

    ctx.say("✅ User removed from the blacklist").await?;
    Ok(())
}
