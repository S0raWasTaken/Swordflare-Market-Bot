use crate::{
    Context, Error, Res,
    commands::{
        language::set_language,
        list_items::list_items,
        moderation::{
            blacklist_user, list_blacklisted_users, mark_as_invalid, pause_bot,
            resume_bot, unblacklist_user,
        },
        new_auction::new_auction,
        new_trade::new_trade,
    },
    database::Data,
};

mod language;
mod list_items;
mod moderation;
mod new_auction;
mod new_trade;

pub fn commands() -> Vec<poise::Command<Data, Error>> {
    vec![
        list_items(),
        new_trade(),
        set_language(),
        mark_as_invalid(),
        blacklist_user(),
        unblacklist_user(),
        list_blacklisted_users(),
        new_auction(),
        pause_bot(),
        resume_bot(),
    ]
}

pub fn check_if_paused(ctx: Context<'_>, locale: &str) -> Res<()> {
    if ctx.data().is_paused() {
        return Err(t!("error.bot_paused", locale = locale).into());
    }
    Ok(())
}

pub async fn check_if_blacklisted(ctx: Context<'_>, locale: &str) -> Res<()> {
    if !is_bot_admin(ctx).await?
        && ctx.data().blacklist.read(|db| db.contains(&ctx.author().id))?
    {
        return Err(t!("error.blacklisted", locale = locale).into());
    }

    Ok(())
}

pub async fn is_bot_admin(ctx: Context<'_>) -> Res<bool> {
    let Some(member) = ctx.author_member().await else {
        return Ok(false);
    };

    let has_admin_role =
        member.roles.iter().any(|r| r == &ctx.data().admin_role);

    if let Some(member_permissions) = member.permissions {
        Ok(has_admin_role || member_permissions.administrator())
    } else {
        Ok(has_admin_role)
    }
}
