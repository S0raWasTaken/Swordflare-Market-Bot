use std::fmt::Write;

use crate::{
    Context, Error, Res,
    commands::{
        language::set_language,
        list_items::list_items,
        moderation::{
            blacklist_user, list_blacklisted_users, list_reports,
            mark_as_invalid, pause_bot, resume_bot, unblacklist_user,
        },
        new_auction::new_auction,
        new_trade::new_trade,
    },
    database::Data,
};

mod language;
mod list_items;
mod moderation;
pub mod new_auction;
pub mod new_trade;

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
        list_reports(),
    ]
}

// ─ Common ─────────────────────────────────────────────────────────────────────

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

// ─ Helpers ────────────────────────────────────────────────────────────────────

pub fn trim_multiline_string(length: usize, string: &mut String) {
    if string.chars().count() <= length {
        return;
    }

    let mut lines = char_prefix(string, length).lines().collect::<Vec<_>>();
    let total = string.lines().count();

    lines.truncate(lines.len().saturating_sub(2));

    let skipped = total - lines.len();

    let mut trimmed = lines.join("\n");
    write!(trimmed, "\n... {skipped} left").ok();

    if trimmed.chars().count() > length {
        trimmed = char_prefix(&trimmed, length).to_string();
    }

    *string = trimmed;
}

fn char_prefix(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}
