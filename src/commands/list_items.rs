use poise::serenity_prelude as serenity;
use std::fmt::Write;

use crate::items::{
    Category::{ActiveSkill, Armor, Aura, Material, PassiveSkill, Weapon},
    ITEMS,
};
use crate::{Context, Res};

#[poise::command(slash_command)]
pub async fn list_items(ctx: Context<'_>) -> Res<()> {
    let categories = [Weapon, Armor, PassiveSkill, ActiveSkill, Material, Aura];

    let embeds: Vec<serenity::CreateEmbed> = categories
        .iter()
        .map(|category| {
            let description = ITEMS
                .iter()
                .filter(|i| matches!(&i.category, c if c == category))
                .fold(String::new(), |mut acc, i| {
                    writeln!(acc, "**{}** — {:?}", i.name, i.rarity).unwrap();
                    acc
                });

            serenity::CreateEmbed::default()
                .title(format!("{category:?}"))
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

    msg.message().await?.delete(ctx.serenity_context()).await?;

    Ok(())
}
