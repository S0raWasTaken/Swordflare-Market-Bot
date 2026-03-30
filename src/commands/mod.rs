use crate::{
    Error,
    commands::{
        language::set_language, list_items::list_items, new_trade::new_trade,
    },
    database::Data,
};

mod language;
mod list_items;
mod new_trade;

pub fn commands() -> Vec<poise::Command<Data, Error>> {
    vec![list_items(), new_trade(), set_language()]
}
