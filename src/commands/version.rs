#![allow(
    clippy::unreadable_literal,
    reason = "hex colours are perfectly readable as is"
)]
use poise::{
    CreateReply,
    serenity_prelude::{CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter},
};
use proc_macro::build_info;

use crate::{Context, Res, START_TIME};

/// Displays my version information
#[poise::command(slash_command)]
pub async fn version(ctx: Context<'_>) -> Res<()> {
    ctx.defer_ephemeral().await?;
    let (version, branch_name, commit_hash, built_at) = build_info!();
    let repo_url = "https://github.com/S0raWasTaken/Swordflare-Market-Bot";

    let branch = format!("[{branch_name}]({repo_url}/tree/{branch_name})");
    let commit = format!("[{commit_hash}]({repo_url}/commit/{commit_hash})");

    let thumbnail = ctx
        .framework()
        .bot_id
        .to_user(ctx)
        .await?
        .avatar_url()
        .unwrap_or_default();
    let gh_favicon =
        "https://github.githubassets.com/favicons/favicon-dark.png";

    let s0ra_avatar_url = "https://cdn.discordapp.com/avatars/319637457907875841/892c9b819388dbf7929ebf1712b508d9.png?size=256";
    let footer =
        CreateEmbedFooter::new(r#"s0ra__ : "devs, please fix your game""#)
            .icon_url(s0ra_avatar_url);

    let embed = CreateEmbed::new()
        .colour(0x770077)
        .author(
            CreateEmbedAuthor::new("Check out my source code")
                .icon_url(gh_favicon)
                .url(repo_url),
        )
        .title(format!("Swordflare Market v{version}"))
        .thumbnail(thumbnail)
        .field("Branch", branch, true)
        .field("Commit", commit, true)
        .field(" ", " ", false) // yeah, padding, blame discord.
        .field("Started", format!("<t:{}:R>", *START_TIME), true)
        .field("Built at", built_at, true)
        .footer(footer);

    ctx.send(CreateReply::default().embed(embed)).await?;

    Ok(())
}
