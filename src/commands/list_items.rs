use poise::serenity_prelude as serenity;
use std::fmt::Write;

use crate::{Context, Res, print_err};
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
#[poise::command(slash_command)]
pub async fn list_items(ctx: Context<'_>) -> Res<()> {
    let locale = &get_user_locale(ctx.data(), ctx.author().id);
    let categories =
        [Weapon, Armor, PassiveSkill, ActiveSkill, Material, Aura, Shard];

    let embeds: Vec<serenity::CreateEmbed> = categories
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

            serenity::CreateEmbed::default()
                .title(category.display(locale))
                .description(description)
        })
        .collect();

    let mut page = 0usize;

    let make_reply = |page: usize| {
        let buttons = serenity::CreateActionRow::Buttons(vec![
            serenity::CreateButton::new("prev").label("◀").disabled(page == 0),
            serenity::CreateButton::new("next")
                .label("▶")
                .disabled(page == embeds.len() - 1),
        ]);
        poise::CreateReply::default()
            .embed(embeds[page].clone())
            .components(vec![buttons])
    };

    let msg = ctx.send(make_reply(page)).await?;

    while let Some(interaction) = msg
        .message()
        .await?
        .await_component_interaction(ctx.serenity_context())
        .timeout(std::time::Duration::from_mins(1))
        .await
    {
        match interaction.data.custom_id.as_str() {
            "prev" => page = page.saturating_sub(1),
            "next" => page = (page + 1).min(embeds.len() - 1),
            _ => {}
        }

        interaction
            .create_response(
                ctx.serenity_context(),
                serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::default()
                        .embed(embeds[page].clone())
                        .components(
                            make_reply(page).components.unwrap_or_default(),
                        ),
                ),
            )
            .await?;
    }

    msg.message()
        .await?
        .delete(ctx.serenity_context())
        .await
        .inspect_err(print_err)
        .ok();

    Ok(())
}
