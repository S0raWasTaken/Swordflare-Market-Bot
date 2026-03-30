use crate::{Context, Res, database::supported_locale::SupportedLocale};

#[poise::command(slash_command)]
pub async fn set_language(
    ctx: Context<'_>,
    #[description = "Your preferred language"] language: SupportedLocale,
) -> Res<()> {
    ctx.defer_ephemeral().await?;

    ctx.data().languages.write(|db| {
        db.insert(ctx.author().id, language);
    })?;

    ctx.data().languages.save()?;

    ctx.reply(t!("set_language.success", locale = &language.to_locale()))
        .await?;
    Ok(())
}
