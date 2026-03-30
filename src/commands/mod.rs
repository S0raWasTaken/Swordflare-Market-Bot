use crate::{
    Context, Error, Res,
    commands::{
        language::set_language,
        list_items::list_items,
        moderation::{blacklist_user, mark_as_invalid, unblacklist_user},
        new_trade::new_trade,
    },
    database::Data,
};

mod language;
mod list_items;
mod moderation;
mod new_trade;

pub fn commands() -> Vec<poise::Command<Data, Error>> {
    vec![
        list_items(),
        new_trade(),
        set_language(),
        mark_as_invalid(),
        blacklist_user(),
        unblacklist_user(),
    ]
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
        Ok(member_permissions.administrator() || has_admin_role)
    } else {
        Ok(has_admin_role)
    }
}
