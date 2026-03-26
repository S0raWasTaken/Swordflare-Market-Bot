use poise::serenity_prelude::{self as serenity};

use crate::{
    Error, Res, database::Data,
    event_handler::buy_interaction::handle_buy_interaction,
};

mod buy_interaction;

pub async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Res<()> {
    if let serenity::FullEvent::InteractionCreate { interaction } = event
        && let serenity::Interaction::Component(component) = interaction
    {
        let custom_id = component.data.custom_id.as_str();
        if custom_id.starts_with("buy_") {
            handle_buy_interaction(ctx, component, data).await?;
        }
    }
    Ok(())
}
