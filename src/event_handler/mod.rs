use poise::serenity_prelude::{
    ComponentInteraction, Context as SerenityContext, FullEvent, Interaction,
    Permissions,
};

use crate::{
    Error, Res,
    database::{Data, supported_locale::get_user_locale},
    event_handler::buttons::{
        auction_cancel::handle_auction_cancel, bid::handle_bid,
        buy::handle_buy, edit::handle_edit, interaction_response,
        refresh::handle_refresh, report::handle_report,
    },
};

pub mod buttons;
pub mod confirm_flow;

macro_rules! match_prefix {
    ($matched:expr, $($starts_with:expr => $fun:expr),*) => {
        match $matched {
            $(
                id if id.starts_with($starts_with) => $fun,
            )*
            _ => return Ok(())
        }
    };
}

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

        let result = match_prefix! {
            custom_id,
            "buy_" => Box::pin(handle_buy(ctx, component, data)).await,
            "bid_" => handle_bid(ctx, component, data).await,

            "edit_" => handle_edit(ctx, component, data).await,
            "refresh_" => handle_refresh(ctx, component, data).await,
            "report_" => handle_report(ctx, component, data).await,

            "au_cancel_" => handle_auction_cancel(ctx, component, data).await
        };

        if let Err(e) = result {
            component
                .create_response(
                    ctx,
                    interaction_response(&e.to_string(), true),
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
        let locale = get_user_locale(ctx, data, user_id).await;
        interaction
            .create_response(
                ctx,
                interaction_response(
                    &t!("error.blacklisted", locale = locale),
                    true,
                ),
            )
            .await?;
        return Ok(true);
    }

    Ok(false)
}
