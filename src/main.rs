#![warn(clippy::pedantic)]

use std::env;

use dotenv::dotenv;
use poise::{
    Framework, FrameworkError, FrameworkOptions,
    samples::register_globally,
    serenity_prelude::{ClientBuilder, GatewayIntents},
};

use crate::{commands::commands, database::Data, event_handler::event_handler};

mod commands;
mod database;
mod event_handler;
mod items;
mod macros;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

type Res<T> = Result<T, Error>;

#[tokio::main]
async fn main() -> Res<()> {
    dotenv()?;
    let intents = GatewayIntents::non_privileged();

    let token = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN must be set");

    let trading_channel_id =
        env::var("TRADING_CHANNEL_ID").expect("TRADING_CHANNEL_ID must be set");
    let interaction_menu_channel_id = env::var("INTERACTION_MENU_CHANNEL_ID")
        .expect("INTERACTION_CHANNEL_MENU_ID must be set");

    let data = Data::new(&trading_channel_id, &interaction_menu_channel_id)?;

    let mut client =
        ClientBuilder::new(token, intents).framework(framework(data)).await?;

    client.start().await?;

    Ok(())
}

async fn on_error(error: FrameworkError<'_, Data, Error>) {
    if let FrameworkError::Command { error, ctx, .. } = error {
        ctx.send(embed! {
                title: format!("Error in command `/{}`", ctx.command().name),
                description: format!(
                    "```diff\n- {}```",
                    error.to_string().replace('\n', "\n- ").trim()
                ),
                ephemeral: true,
                mentions: None,
                reply: true,
        })
        .await
        .ok();
    } else {
        poise::builtins::on_error(error).await.ok();
    }
}

fn framework(data: Data) -> Framework<Data, Error> {
    let options = FrameworkOptions {
        commands: commands(),
        event_handler: |ctx, event, framework, data| {
            Box::pin(event_handler(ctx, event, framework, data))
        },
        on_error: |e| Box::pin(on_error(e)),
        ..Default::default()
    };

    Framework::builder()
        .options(options)
        .setup(|ctx, ready, framework| {
            Box::pin(async move {
                println!("{} is on!", ready.user.name);
                register_globally(ctx, &framework.options().commands).await?;
                Ok(data)
            })
        })
        .build()
}
