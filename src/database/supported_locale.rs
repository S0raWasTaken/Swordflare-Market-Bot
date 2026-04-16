use poise::{
    SlashArgError, SlashArgument,
    serenity_prelude::{self as serenity, CacheHttp, UserId},
};
use serde::{Deserialize, Serialize};

use crate::{Res, database::Data};

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
#[expect(non_camel_case_types)]
pub enum SupportedLocale {
    #[default]
    en_US,
    ko_KR,
    en_BRAILLE,
    en_LOL,
    en_PIRATE,
    en_REV,
}

#[allow(clippy::enum_glob_use)]
use SupportedLocale::*;

impl SupportedLocale {
    pub fn to_locale(self) -> &'static str {
        match self {
            en_US => "en-US",
            ko_KR => "ko-KR",
            en_BRAILLE => "en-BRAILLE",
            en_LOL => "en-LOL",
            en_PIRATE => "en-PIRATE",
            en_REV => "en-REV",
        }
    }

    pub fn from_locale(locale: &str) -> Res<Self> {
        match locale {
            "en-US" | "en" => Ok(en_US),
            "ko-KR" | "ko" => Ok(ko_KR),
            "en-BRAILLE" => Ok(en_BRAILLE),
            "en-LOL" => Ok(en_LOL),
            "en-PIRATE" => Ok(en_PIRATE),
            "en-REV" => Ok(en_REV),
            _ => Err("Invalid or unsupported locale".into()),
        }
    }

    #[inline]
    pub fn from_locale_fallback(locale: &str) -> Self {
        Self::from_locale(locale).unwrap_or(en_US)
    }

    pub fn korean_or_english(self) -> Self {
        if matches!(self, ko_KR) { self } else { en_US }
    }
}

impl SlashArgument for SupportedLocale {
    fn create(
        builder: serenity::CreateCommandOption,
    ) -> serenity::CreateCommandOption {
        builder
            .kind(serenity::CommandOptionType::String)
            .add_string_choice("English", "en-US")
            .add_string_choice("한국어", "ko-KR")
            // Meme ones
            .add_string_choice("Pirate", "en-PIRATE")
            .add_string_choice("Lolcat", "en-LOL")
            .add_string_choice("Braille", "en-BRAILLE")
            .add_string_choice("Reversed", "en-REV")
    }

    fn extract<'life0, 'life1, 'life2, 'life3, 'async_trait>(
        _: &'life0 serenity::Context,
        _: &'life1 serenity::CommandInteraction,
        value: &'life2 serenity::ResolvedValue<'life3>,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = Result<Self, SlashArgError>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        'life2: 'async_trait,
        'life3: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            let serenity::ResolvedValue::String(s) = value else {
                return Err(SlashArgError::new_command_structure_mismatch(
                    "expected string",
                ));
            };
            SupportedLocale::from_locale(s).map_err(|_| {
                SlashArgError::new_command_structure_mismatch(
                    "unsupported locale",
                )
            })
        })
    }
}

pub async fn get_user_locale(
    http: impl CacheHttp,
    data: &Data,
    id: UserId,
) -> &'static str {
    if let Some(locale) = data
        .languages
        .borrow_data()
        .ok()
        .and_then(|languages| languages.get(&id).map(|l| l.to_locale()))
    {
        return locale;
    }

    let supported_locale =
        id.to_user(http).await.ok().and_then(|user| user.locale).map_or_else(
            || SupportedLocale::en_US,
            |l| SupportedLocale::from_locale_fallback(&l),
        );
    data.languages
        .write(|db| {
            db.insert(id, supported_locale);
        })
        .ok();

    supported_locale.to_locale()
}
