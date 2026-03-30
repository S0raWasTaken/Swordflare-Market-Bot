#![warn(clippy::pedantic)]

#[macro_use]
extern crate rust_i18n;

pub use rust_i18n::t;
use std::{fmt::Display, sync::LazyLock};

use dotenv::dotenv;
use poise::{
    Framework, FrameworkError, FrameworkOptions,
    samples::register_globally,
    serenity_prelude::{ClientBuilder, GatewayIntents},
};
use tokio::time::interval;

use crate::{
    cleanup::cleanup, commands::commands, database::Data,
    event_handler::event_handler, magic_numbers::DATABASE_CLEANUP_INTERVAL,
};

mod cleanup;
mod commands;
mod database;
mod event_handler;
mod item_name;
mod items;
mod macros;
mod magic_numbers;
mod post;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

type Res<T> = Result<T, Error>;

pub static TRADING_SERVER_LINK: LazyLock<String> = LazyLock::new(|| {
    std::env::var("TRADING_PRIVATE_SERVER_LINK")
        .expect("TRADING_PRIVATE_SERVER_LINK must be set")
});

i18n!("locales", fallback = "en-US");

#[tokio::main]
async fn main() -> Res<()> {
    dotenv()?;

    let (
        token,
        english_posting_channel_id,
        korean_posting_channel_id,
        english_menu_channel_id,
        korean_menu_channel_id,
    ) = get_vars!(
        "DISCORD_TOKEN",
        "ENGLISH_POSTING_CHANNEL_ID",
        "KOREAN_POSTING_CHANNEL_ID",
        "ENGLISH_MENU_CHANNEL_ID",
        "KOREAN_MENU_CHANNEL_ID"
    );

    let data = Data::new(
        &english_posting_channel_id,
        &korean_posting_channel_id,
        &english_menu_channel_id,
        &korean_menu_channel_id,
    )?;

    let mut client =
        ClientBuilder::new(token, GatewayIntents::non_privileged())
            .framework(framework(data))
            .await?;

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

                let data_clone = data.clone(); // Custom clone, Arc inside
                let ctx_clone = ctx.clone();
                tokio::spawn(async move {
                    let mut interval = interval(DATABASE_CLEANUP_INTERVAL);
                    loop {
                        interval.tick().await;
                        cleanup(&ctx_clone, &data_clone).await;
                    }
                });

                Ok(data)
            })
        })
        .build()
}

pub fn print_err<E: Display>(e: &E) {
    log!(e "{e}");
}
