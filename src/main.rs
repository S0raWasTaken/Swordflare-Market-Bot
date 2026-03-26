#![warn(clippy::pedantic)]

use dotenv::dotenv;
use poise::{
    Framework, FrameworkError, FrameworkOptions,
    samples::register_globally,
    serenity_prelude::{ClientBuilder, GatewayIntents},
};

use crate::database::TradingDatabase;

mod database;
mod items;
mod macros;

struct Data {
    trades: TradingDatabase,
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

type Res<T> = Result<T, Error>;

#[tokio::main]
async fn main() -> Res<()> {
    dotenv()?;
    let intents = GatewayIntents::non_privileged();

    let trades = TradingDatabase::load_from_path_or_default("trading_db.yml")?;

    let mut client =
        ClientBuilder::new(std::env::var("DISCORD_TOKEN")?, intents)
            .framework(framework(trades))
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

fn framework(trades: TradingDatabase) -> Framework<Data, Error> {
    let options = FrameworkOptions {
        on_error: |e| Box::pin(on_error(e)),
        ..Default::default()
    };

    Framework::builder()
        .options(options)
        .setup(|ctx, ready, framework| {
            Box::pin(async move {
                println!("{} is on!", ready.user.name);
                register_globally(ctx, &framework.options().commands).await?;
                Ok(Data { trades })
            })
        })
        .build()
}
