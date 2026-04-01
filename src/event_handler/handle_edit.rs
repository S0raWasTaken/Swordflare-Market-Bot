use std::{
    borrow::Cow,
    ops::ControlFlow::{Break, Continue},
};
type ControlFlow<T> = std::ops::ControlFlow<(), T>;

use poise::serenity_prelude::{
    self as serenity, ActionRowComponent, ComponentInteraction,
    CreateActionRow, CreateInputText, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateModal, InputTextStyle,
    ModalInteraction, ModalInteractionCollector, User, UserId,
};

use crate::{
    Error, Res,
    database::{
        Data,
        supported_locale::{SupportedLocale, get_user_locale},
        trade_db::Trade,
    },
    event_handler::buy_interaction::fetch_trade,
    magic_numbers::TRADE_CONFIRMATION_TIMEOUT,
    post::update_post,
};

macro_rules! break_or {
    ($expr:expr) => {
        match $expr {
            Continue(v) => v,
            _ => return Ok(()),
        }
    };
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn handle_edit(
    ctx: &serenity::Context,
    interaction: &ComponentInteraction,
    data: &Data,
) -> Res<()> {
    let edit_ctx = EditContext::new(interaction, ctx, data);

    let (trade_id, trade) = break_or!(resolve_trade(&edit_ctx).await?);
    let (lots, modal) = break_or!(prompt_edit(&edit_ctx, &trade).await?);

    update_trade(&edit_ctx, trade_id, lots).await?;
    finish(&edit_ctx, &modal).await?;

    Ok(())
}

// ── Steps ─────────────────────────────────────────────────────────────────────

async fn resolve_trade(
    edit_ctx: &EditContext<'_>,
) -> Res<ControlFlow<(u64, Trade)>> {
    let locale = &edit_ctx.locale();
    let trade = fetch_trade(edit_ctx.data, edit_ctx.trade_id()?, locale)?;

    if !edit_ctx.interaction_user_is_seller(trade.1.seller) {
        edit_ctx
            .reply_ephemeral(&t!("edit.error.not_seller", locale = locale))
            .await?;
        return Ok(Break(()));
    }

    Ok(Continue(trade))
}

type Lots = u16;
async fn prompt_edit(
    edit_ctx: &EditContext<'_>,
    trade: &Trade,
) -> Res<ControlFlow<(Lots, ModalInteraction)>> {
    let locale = &edit_ctx.locale();
    let custom_id = format!("quantity_{}", edit_ctx.trade_id()?);

    edit_ctx
        .create_response(CreateInteractionResponse::Modal(
            modal(&custom_id, &t!("edit.modal.title", locale = locale))
                .components(vec![input_action_row(input_text(
                    &t!("edit.modal.input_label", locale = locale),
                    "quantity",
                    &t!("edit.modal.placeholder", locale = locale),
                ))]),
        ))
        .await?;

    let Some(modal) =
        modal_collector(edit_ctx.ctx, edit_ctx.user().id, custom_id).await
    else {
        return Ok(Break(()));
    };

    let parsed = parse_input_modal(&modal, locale);

    let lots = match parsed {
        Ok(stock) => stock / trade.quantity,
        Err(e) => {
            modal
                .create_response(edit_ctx.ctx, interaction_response(&e, true))
                .await?;
            return Ok(Break(()));
        }
    };

    // lots == 0 is valid, it means the seller is out of stock.

    Ok(Continue((lots, modal)))
}

async fn update_trade(
    edit_ctx: &EditContext<'_>,
    trade_id: u64,
    lots: u16,
) -> Res<()> {
    let data = edit_ctx.data;

    data.trades.write(|db| {
        db.get_mut(trade_id)
            .ok_or(format!("Trade not found: {trade_id}"))?
            .stock = lots;
        Ok::<(), Error>(())
    })??;

    let en_result = update_post(
        edit_ctx.ctx,
        edit_ctx.data,
        trade_id,
        SupportedLocale::en_US,
    )
    .await;
    let ko_result = update_post(
        edit_ctx.ctx,
        edit_ctx.data,
        trade_id,
        SupportedLocale::ko_KR,
    )
    .await;

    en_result?;
    ko_result?;

    Ok(())
}

async fn finish(
    edit_ctx: &EditContext<'_>,
    modal: &ModalInteraction,
) -> Result<(), serenity::Error> {
    modal
        .create_response(
            edit_ctx.ctx,
            interaction_response(
                &t!("edit.success", locale = &edit_ctx.locale()),
                true,
            ),
        )
        .await
}

// ── Helpers ───────────────────────────────────────────────────────────────────

pub fn parse_input_modal(
    modal: &ModalInteraction,
    locale: &str,
) -> Result<u16, Cow<'static, str>> {
    modal
        .data
        .components
        .iter()
        .flat_map(|row| row.components.iter())
        .find_map(|component| {
            if let ActionRowComponent::InputText(text) = component {
                text.value.as_deref()
            } else {
                None
            }
        })
        .ok_or(t!("edit.error.missing_stock_input", locale = locale))
        .and_then(|value| {
            value
                .parse::<u16>()
                .map_err(|_| t!("error.invalid_number", locale = locale))
        })
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
        .max_length(5)
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

struct EditContext<'a> {
    pub interaction: &'a ComponentInteraction,
    pub ctx: &'a serenity::Context,
    pub data: &'a Data,
}

impl<'a> EditContext<'a> {
    pub fn new(
        interaction: &'a ComponentInteraction,
        ctx: &'a serenity::Context,
        data: &'a Data,
    ) -> Self {
        Self { interaction, ctx, data }
    }

    pub fn user(&self) -> &'a User {
        &self.interaction.user
    }

    pub fn trade_id(&self) -> Res<u64> {
        Ok(self
            .interaction
            .data
            .custom_id
            .strip_prefix("edit_")
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
