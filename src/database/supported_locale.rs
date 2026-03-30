use poise::{
    SlashArgError, SlashArgument,
    serenity_prelude::{self as serenity, UserId},
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
}

impl SupportedLocale {
    pub fn to_locale(self) -> &'static str {
        match self {
            SupportedLocale::en_US => "en-US",
            SupportedLocale::ko_KR => "ko-KR",
        }
    }

    pub fn from_locale(locale: &str) -> Res<Self> {
        match locale {
            "en-US" => Ok(Self::en_US),
            "ko-KR" => Ok(Self::ko_KR),
            _ => Err("Invalid or unsupported locale".into()),
        }
    }

    #[inline]
    pub fn from_locale_fallback(locale: &str) -> Self {
        Self::from_locale(locale).unwrap_or(Self::en_US)
    }

    pub fn korean_or_english(self) -> Self {
        if matches!(self, Self::ko_KR) { self } else { Self::en_US }
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

pub fn get_user_locale(data: &Data, id: UserId) -> String {
    data.languages
        .borrow_data()
        .ok()
        .and_then(|languages| {
            languages.get(&id).map(|l| l.to_locale().to_string())
        })
        .unwrap_or_else(|| "en-US".to_string())
}
