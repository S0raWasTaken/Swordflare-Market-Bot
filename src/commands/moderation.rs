use std::{fmt::Write, time::Duration};

use poise::{
    CreateReply,
    serenity_prelude::{
        CreateActionRow, CreateButton, CreateEmbed, CreateEmbedFooter,
        CreateInteractionResponse, CreateInteractionResponseMessage, Message,
        MessageId, User,
    },
};

use crate::{
    Context, Res,
    cleanup::clean_database,
    commands::{is_bot_admin, trim_multiline_string},
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
            auction.delete_messages(ctx.serenity_context(), ctx.data()).await?;
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

#[poise::command(
    context_menu_command = "List Reports",
    check = "is_bot_admin",
    interaction_context = "Guild"
)]
pub async fn list_reports(ctx: Context<'_>, msg: Message) -> Res<()> {
    ctx.defer_ephemeral().await?;

    let Some(post_reports) = ctx.data().trades.read(|db| {
        db.iter().find_map(|entry| {
            if entry.1.english_message_id.is_eq(msg.id)
                || entry.1.korean_message_id.is_eq(msg.id)
            {
                Some(entry.1.reports.clone())
            } else {
                None
            }
        })
    })?
    else {
        ctx.say("Post was not found in the database").await?;
        return Ok(());
    };

    if post_reports.is_empty() {
        ctx.say("There are no reports for this post").await?;
        return Ok(());
    }

    let mut full_list = String::new();

    for (user, report_message) in post_reports {
        writeln!(full_list, "<@{user}>: `{report_message}`")?;
    }

    trim_multiline_string(2000, &mut full_list);

    ctx.say(full_list).await?;

    Ok(())
}

#[poise::command(
    slash_command,
    check = "is_bot_admin",
    interaction_context = "Guild"
)]
pub async fn list_blacklisted_users(ctx: Context<'_>) -> Res<()> {
    let blacklisted_users = {
        ctx.data()
            .blacklist
            .borrow_data()?
            .iter()
            .map(|id| format!("<@{id}>: `{id}`"))
            .collect::<Vec<_>>()
    }
    .chunks(10)
    .map(|chunk| {
        let description = chunk.join("\n");
        CreateEmbed::default()
            .title("Blacklisted Users")
            .description(description)
    })
    .collect::<Vec<_>>();

    if blacklisted_users.is_empty() {
        ctx.say("✅ No blacklisted users.").await?;
        return Ok(());
    }

    let max_page_number = blacklisted_users.len() - 1;
    let mut page = 0;

    let make_buttons = |page: usize| {
        CreateActionRow::Buttons(vec![
            CreateButton::new("prev").label("◀").disabled(page == 0),
            CreateButton::new("next")
                .label("▶")
                .disabled(page == max_page_number),
        ])
    };

    let make_reply = |page: usize| {
        CreateReply::default()
            .embed(blacklisted_users[page].clone().footer(
                CreateEmbedFooter::new(format!(
                    "{}/{}",
                    page + 1,
                    max_page_number + 1
                )),
            ))
            .components(vec![make_buttons(page)])
    };

    let msg = ctx.send(make_reply(page)).await?;

    let cached_message = msg.message().await?;

    while let Some(interaction) = cached_message
        .await_component_interaction(ctx.serenity_context())
        .timeout(Duration::from_mins(1))
        .await
    {
        match interaction.data.custom_id.as_str() {
            "prev" => page = page.saturating_sub(1),
            "next" => page = (page + 1).min(max_page_number),
            _ => (),
        }

        interaction
            .create_response(
                ctx.serenity_context(),
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::default()
                        .embed(blacklisted_users[page].clone())
                        .components(vec![make_buttons(page)]),
                ),
            )
            .await?;
    }

    msg.delete(ctx).await?;

    Ok(())
}

/// Pauses bot execution for whatever purpose
#[poise::command(
    slash_command,
    check = "is_bot_admin",
    interaction_context = "Guild"
)]
pub async fn pause_bot(ctx: Context<'_>) -> Res<()> {
    ctx.defer_ephemeral().await?;

    if !ctx.data().pause() {
        return Err("❌ Already paused!".into());
    }

    let guild = ctx.guild_id().ok_or(
        r#"Cannot find guild. 
        Which is weird, because interaction_context = "Guild""#,
    )?;

    guild.edit_nickname(ctx.http(), Some("[PAUSED]")).await?;

    ctx.say("✅ Paused!").await?;
    Ok(())
}

/// Resumes bot execution
#[poise::command(
    slash_command,
    check = "is_bot_admin",
    interaction_context = "Guild"
)]
pub async fn resume_bot(ctx: Context<'_>) -> Res<()> {
    ctx.defer_ephemeral().await?;

    if !ctx.data().resume() {
        return Err("❌ Already running!".into());
    }

    let guild = ctx.guild_id().ok_or(
        r#"Cannot find guild. 
        Which is weird, because interaction_context = "Guild" "#,
    )?;

    guild.edit_nickname(ctx.http(), None).await?;

    ctx.say("✅ Resumed!").await?;
    Ok(())
}
