use poise::serenity_prelude::{
    ComponentInteraction, Context as SerenityContext,
    CreateInteractionResponse, CreateInteractionResponseMessage, FullEvent,
    Interaction,
};

use crate::{
    Error, Res,
    database::{Data, supported_locale::get_user_locale},
    event_handler::buy_interaction::handle_buy_interaction,
};

mod buy_interaction;

pub async fn event_handler(
    ctx: &SerenityContext,
    event: &FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Res<()> {
    if let FullEvent::InteractionCreate { interaction } = event
        && let Interaction::Component(component) = interaction
    {
        if is_blacklisted(ctx, component, data).await? {
            return Ok(());
        }

        let custom_id = component.data.custom_id.as_str();
        if custom_id.starts_with("buy_") {
            handle_buy_interaction(ctx, component, data).await?;
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
    let guild_id = interaction.guild_id.ok_or("Expected guild interaction")?;

    let (member_roles, permissions) = {
        let guild =
            guild_id.to_guild_cached(ctx).ok_or("Guild not in cache")?;
        let member =
            guild.members.get(&user_id).ok_or("Member not in cache")?;
        let channel = guild
            .channels
            .get(&interaction.channel_id)
            .ok_or("Channel not in cache")?;
        let permissions = guild.user_permissions_in(channel, member);
        (member.roles.clone(), permissions)
    };

    if permissions.administrator()
        || member_roles.iter().any(|r| r == &data.admin_role)
    {
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
