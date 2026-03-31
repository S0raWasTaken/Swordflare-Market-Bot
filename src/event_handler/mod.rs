use poise::serenity_prelude::{
    ComponentInteraction, Context as SerenityContext,
    CreateInteractionResponse, CreateInteractionResponseMessage, FullEvent,
    Interaction, Permissions,
};

use crate::{
    Error, Res,
    database::{Data, supported_locale::get_user_locale},
    event_handler::{
        auction_bid::handle_bid_interaction,
        buy_interaction::handle_buy_interaction,
    },
};

mod auction_bid;
mod buy_interaction;
pub mod confirm_flow;

pub async fn event_handler(
    ctx: &SerenityContext,
    event: &FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Res<()> {
    if let FullEvent::InteractionCreate { interaction } = event
        && let Interaction::Component(component) = interaction
    {
        if is_blacklisted(ctx, component, data).await? || data.is_paused() {
            return Ok(());
        }

        let custom_id = component.data.custom_id.as_str();

        let result = match custom_id {
            id if id.starts_with("buy_") => {
                handle_buy_interaction(ctx, component, data).await
            }
            id if id.starts_with("bid_") => {
                handle_bid_interaction(ctx, component, data).await
            }
            _ => return Ok(()),
        };

        if let Err(e) = result {
            component
                .create_response(
                    ctx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::default()
                            .ephemeral(true)
                            .content(format!("❌ {e}")),
                    ),
                )
                .await
                .ok();
        }
    }

    Ok(())
}

async fn is_blacklisted(
    ctx: &SerenityContext,
    interaction: &ComponentInteraction,
    data: &Data,
) -> Res<bool> {
    let user_id = interaction.user.id;

    // DM interactions can't be blacklisted
    let Some(guild_id) = interaction.guild_id else {
        return Ok(false);
    };

    let member = guild_id.member(ctx, user_id).await?;
    let is_exempt = member.roles.iter().any(|r| r == &data.admin_role)
        || member.permissions.is_some_and(Permissions::administrator);

    if is_exempt {
        return Ok(false);
    }

    if data.blacklist.borrow_data()?.contains(&user_id) {
        let locale = get_user_locale(data, user_id);
        interaction
            .create_response(
                ctx,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::default()
                        .ephemeral(true)
                        .content(t!("error.blacklisted", locale = locale)),
                ),
            )
            .await?;
        return Ok(true);
    }

    Ok(false)
}
