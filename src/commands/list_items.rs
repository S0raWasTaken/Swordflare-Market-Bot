use poise::serenity_prelude::{
    CreateActionRow, CreateButton, CreateEmbed, CreateEmbedFooter,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};
use std::fmt::Write;

use crate::commands::check_if_paused;
use crate::{Context, Res};
use crate::{
    database::supported_locale::get_user_locale,
    items::{
        Category::{
            ActiveSkill, Armor, Aura, Material, PassiveSkill, Shard, Weapon,
        },
        ITEMS,
    },
};

/// List all items in game
/// 게임 내 모든 아이템 나열
#[poise::command(slash_command, interaction_context = "Guild")]
pub async fn list_items(ctx: Context<'_>) -> Res<()> {
    let locale = &get_user_locale(ctx.data(), ctx.author().id);
    check_if_paused(ctx, locale)?;

    let categories =
        [Weapon, Armor, PassiveSkill, ActiveSkill, Material, Aura, Shard];

    let embeds: Vec<CreateEmbed> = categories
        .iter()
        .map(|category| {
            let description = ITEMS
                .iter()
                .filter(|i| &i.category == category)
                .fold(String::new(), |mut acc, i| {
                    writeln!(
                        acc,
                        "**{}** — {}",
                        i.name.display(locale),
                        i.rarity.display(locale)
                    )
                    .unwrap();
                    acc
                });

            CreateEmbed::default()
                .title(category.display(locale))
                .description(description)
        })
        .collect();

    let max_page_number = embeds.len() - 1;
    let mut page = 0usize;

    let make_buttons = |page: usize| {
        CreateActionRow::Buttons(vec![
            CreateButton::new("prev").label("◀").disabled(page == 0),
            CreateButton::new("next")
                .label("▶")
                .disabled(page == max_page_number),
        ])
    };

    let make_reply = |page: usize| {
        poise::CreateReply::default()
            .embed(embeds[page].clone().footer(CreateEmbedFooter::new(
                format!("{page}/{max_page_number}"),
            )))
            .components(vec![make_buttons(page)])
    };

    let msg = ctx.send(make_reply(page)).await?;

    let cached_message = msg.message().await?;

    while let Some(interaction) = cached_message
        .await_component_interaction(ctx.serenity_context())
        .timeout(std::time::Duration::from_mins(1))
        .await
    {
        match interaction.data.custom_id.as_str() {
            "prev" => page = page.saturating_sub(1),
            "next" => page = (page + 1).min(max_page_number),
            _ => {}
        }

        interaction
            .create_response(
                ctx.serenity_context(),
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::default()
                        .embed(embeds[page].clone().footer(
                            CreateEmbedFooter::new(format!(
                                "{page}/{max_page_number}"
                            )),
                        ))
                        .components(vec![make_buttons(page)]),
                ),
            )
            .await?;
    }

    msg.delete(ctx).await.ok();

    Ok(())
}
