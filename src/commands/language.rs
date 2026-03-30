use crate::{Context, Res, database::supported_locale::SupportedLocale};

/// Set your language
/// 언어를 설정하세요
#[poise::command(slash_command, interaction_context = "Guild")]
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
