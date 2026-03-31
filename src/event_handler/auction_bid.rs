use poise::serenity_prelude as serenity;

use crate::{
    Data, Res,
    database::supported_locale::{SupportedLocale, get_user_locale},
    magic_numbers::TRADE_CONFIRMATION_TIMEOUT,
    post::update_auction_post,
    print_err,
};

use super::super::database::auction_db::RunningAuction;

#[expect(clippy::too_many_lines)]
pub async fn handle_bid_interaction(
    ctx: &serenity::Context,
    interaction: &serenity::ComponentInteraction,
    data: &Data,
) -> Res<()> {
    let bidder = &interaction.user;
    let locale = get_user_locale(data, bidder.id);

    let auction_id: u64 = interaction
        .data
        .custom_id
        .strip_prefix("bid_")
        .ok_or(t!("error.invalid_custom_id", locale = locale))?
        .parse()?;

    // Resolve what we need before any await
    let (seller_id, is_expired, min_next_bid, currency_name) = {
        let db = data.running_auctions.borrow_data()?;
        let auction = db
            .get(auction_id)
            .ok_or(t!("error.trade_not_found", locale = locale))?;
        (
            auction.seller,
            auction.is_expired(),
            auction.min_next_bid().ok_or(t!(
                "auction.error.max_value_reached",
                locale = locale
            ))?,
            auction.currency_item.name.display(&locale).into_owned(),
        )
    };

    // Guard: seller can't bid on own auction
    if bidder.id == seller_id {
        interaction
            .create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::default()
                        .ephemeral(true)
                        .content(t!("auction.error.self_bid", locale = locale)),
                ),
            )
            .await?;
        return Ok(());
    }

    // Guard: auction expired
    if is_expired {
        interaction
            .create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::default()
                        .ephemeral(true)
                        .content(t!("auction.error.expired", locale = locale)),
                ),
            )
            .await?;
        return Ok(());
    }

    // Show modal
    interaction
        .create_response(
            ctx,
            serenity::CreateInteractionResponse::Modal(
                serenity::CreateModal::new(
                    format!("bid_amount_{auction_id}"),
                    t!("auction.modal.title", locale = locale),
                )
                .components(vec![
                    serenity::CreateActionRow::InputText(
                        serenity::CreateInputText::new(
                            serenity::InputTextStyle::Short,
                            t!(
                                "auction.modal.input_label",
                                locale = locale,
                                min = min_next_bid,
                                currency = currency_name
                            ),
                            "bid_amount",
                        )
                        .min_length(1)
                        .max_length(5)
                        .placeholder(min_next_bid.to_string()),
                    ),
                ]),
            ),
        )
        .await?;

    // Wait for modal submission
    let Some(modal) = serenity::collector::ModalInteractionCollector::new(ctx)
        .author_id(bidder.id)
        .custom_ids(vec![format!("bid_amount_{auction_id}")])
        .timeout(TRADE_CONFIRMATION_TIMEOUT)
        .next()
        .await
    else {
        return Ok(());
    };

    // Parse bid amount
    let amount: u16 = match modal
        .data
        .components
        .iter()
        .flat_map(|r| r.components.iter())
        .find_map(|c| {
            if let serenity::ActionRowComponent::InputText(t) = c {
                t.value.as_deref()
            } else {
                None
            }
        })
        .ok_or(t!("error.missing_lots_input", locale = locale))
        .and_then(|v| {
            v.parse::<u16>()
                .map_err(|_| t!("error.invalid_number", locale = locale))
        }) {
        Ok(a) => a,
        Err(e) => {
            modal
                .create_response(
                    ctx,
                    serenity::CreateInteractionResponse::Message(
                        serenity::CreateInteractionResponseMessage::default()
                            .ephemeral(true)
                            .content(format!("❌ {e}")),
                    ),
                )
                .await?;
            return Ok(());
        }
    };

    // Validate and insert bid
    let bid_accepted = data.running_auctions.write(|db| {
        let Some(auction) = db.get_mut(auction_id) else {
            return false;
        };
        if !auction.is_valid_bid(bidder.id, amount) {
            return false;
        }
        auction.bids.insert(bidder.id, amount);
        true
    })?;

    if !bid_accepted {
        // Re-read min_next_bid for a fresh error message
        let current_min = data
            .running_auctions
            .borrow_data()?
            .get(auction_id)
            .and_then(RunningAuction::min_next_bid)
            .unwrap_or(min_next_bid);

        modal
            .create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::default()
                        .ephemeral(true)
                        .content(t!(
                            "auction.error.invalid_bid",
                            locale = locale,
                            min = current_min,
                            currency = currency_name
                        )),
                ),
            )
            .await?;
        return Ok(());
    }

    data.running_auctions.save()?;

    // Acknowledge and update both posts
    modal
        .create_response(
            ctx,
            serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::default()
                    .ephemeral(true)
                    .content(t!(
                        "auction.bid.accepted",
                        locale = locale,
                        amount = amount,
                        currency = currency_name
                    )),
            ),
        )
        .await?;

    let ko_update_result =
        update_auction_post(ctx, data, auction_id, SupportedLocale::ko_KR)
            .await
            .inspect_err(print_err);
    update_auction_post(ctx, data, auction_id, SupportedLocale::en_US)
        .await
        .inspect_err(print_err)?;

    ko_update_result?;

    Ok(())
}
