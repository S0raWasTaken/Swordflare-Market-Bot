use std::ops::ControlFlow::{Break, Continue};

use poise::serenity_prelude::{
    self as serenity, ActionRowComponent, ComponentInteraction,
    CreateActionRow, CreateInputText, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateModal, InputTextStyle,
    ModalInteraction, ModalInteractionCollector, User, UserId,
};

use crate::database::supported_locale::SupportedLocale;
use crate::post::update_post;
use crate::{
    Res,
    database::{Data, supported_locale::get_user_locale, trade_db::Trade},
    magic_numbers::TRADE_CONFIRMATION_TIMEOUT,
};

pub mod bid;
pub mod buy;
pub mod edit;
pub mod refresh;
pub mod report;

// ── Common ────────────────────────────────────────────────────────────────────

pub type ControlFlow<T> = std::ops::ControlFlow<(), T>;

pub async fn resolve_trade(
    button_ctx: &ButtonContext<'_>,
    error_condition: impl Fn(UserId) -> Option<String>,
) -> Res<ControlFlow<(u64, Trade)>> {
    let locale = &button_ctx.locale();
    let trade_id = button_ctx.trade_id()?;
    let trade = fetch_trade(button_ctx.data, trade_id, locale)?;

    if let Some(error_msg) = error_condition(trade.seller) {
        button_ctx.reply_ephemeral(&error_msg).await?;
        return Ok(Break(()));
    }

    Ok(Continue((trade_id, trade)))
}

async fn update_posts(
    button_ctx: &ButtonContext<'_>,
    trade_id: u64,
) -> Res<()> {
    let (en_result, ko_result) = tokio::join! {
        update_post(button_ctx.ctx, button_ctx.data, trade_id, SupportedLocale::en_US),
        update_post(button_ctx.ctx, button_ctx.data, trade_id, SupportedLocale::ko_KR)
    };

    en_result?;
    ko_result?;
    Ok(())
}

pub fn fetch_trade(data: &Data, trade_id: u64, locale: &str) -> Res<Trade> {
    data.trades
        .borrow_data()?
        .get(trade_id)
        .ok_or(t!("error.trade_not_found", locale = locale))
        .cloned()
        .map_err(Into::into)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

#[macro_export]
macro_rules! break_or {
    ($expr:expr) => {
        match $expr {
            std::ops::ControlFlow::Continue(v) => v,
            _ => return Ok(()),
        }
    };
}

pub fn parse_number_in_modal(
    modal: &ModalInteraction,
    locale: &str,
    error_msg: String,
) -> Res<u64> {
    Ok(parse_modal(modal, error_msg).and_then(|value| {
        Ok(value
            .parse::<u64>()
            .map_err(|_| t!("error.invalid_number", locale = locale))?)
    })?)
}

pub fn parse_modal(
    modal: &ModalInteraction,
    error_msg: String,
) -> Result<String, String> {
    modal
        .data
        .components
        .iter()
        .flat_map(|row| row.components.iter())
        .find_map(|component| {
            if let ActionRowComponent::InputText(text) = component {
                text.value.clone()
            } else {
                None
            }
        })
        .ok_or(error_msg)
}

pub async fn modal_collector(
    ctx: &serenity::Context,
    author_id: UserId,
    custom_id: String,
) -> Option<ModalInteraction> {
    ModalInteractionCollector::new(ctx)
        .author_id(author_id)
        .custom_ids(vec![custom_id])
        .timeout(TRADE_CONFIRMATION_TIMEOUT)
        .next()
        .await
}

#[inline]
pub fn modal(custom_id: &str, title: &str) -> CreateModal {
    CreateModal::new(custom_id, title)
}

#[inline]
pub fn input_action_row(input_text: CreateInputText) -> CreateActionRow {
    CreateActionRow::InputText(input_text)
}

#[inline]
pub fn input_text(
    label: &str,
    custom_id: &str,
    placeholder: &str,
) -> CreateInputText {
    CreateInputText::new(InputTextStyle::Short, label, custom_id)
        .placeholder(placeholder)
        .min_length(1)
        .max_length(19)
}

#[inline]
pub fn interaction_response(
    content: &str,
    ephemeral: bool,
) -> CreateInteractionResponse {
    CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::default()
            .ephemeral(ephemeral)
            .content(content),
    )
}

// ── Data types ────────────────────────────────────────────────────────────────

pub struct ButtonContext<'a> {
    pub interaction: &'a ComponentInteraction,
    pub ctx: &'a serenity::Context,
    pub data: &'a Data,
    pub prefix: &'a str,
}

impl<'a> ButtonContext<'a> {
    pub fn new(
        interaction: &'a ComponentInteraction,
        ctx: &'a serenity::Context,
        data: &'a Data,
        prefix: &'a str,
    ) -> Self {
        Self { interaction, ctx, data, prefix }
    }

    pub fn user(&self) -> &'a User {
        &self.interaction.user
    }

    pub fn trade_id(&self) -> Res<u64> {
        Ok(self
            .interaction
            .data
            .custom_id
            .strip_prefix(self.prefix)
            .ok_or(t!("error.invalid_custom_id", locale = &self.locale()))?
            .parse()?)
    }

    #[inline]
    pub fn interaction_user_is_seller(&self, seller: UserId) -> bool {
        self.user().id == seller
    }

    pub async fn reply_ephemeral(
        &self,
        content: &str,
    ) -> Result<(), serenity::Error> {
        self.create_response(interaction_response(content, true)).await
    }

    pub fn create_response(
        &self,
        response: CreateInteractionResponse,
    ) -> impl Future<Output = Result<(), serenity::Error>> {
        self.interaction.create_response(self.ctx, response)
    }

    pub fn locale(&self) -> String {
        get_user_locale(self.data, self.user().id)
    }
}
