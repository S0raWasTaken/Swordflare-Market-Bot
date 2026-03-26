use crate::{Error, commands::list_items::list_items, database::Data};

mod list_items;

pub fn commands() -> Vec<poise::Command<Data, Error>> {
    vec![list_items()]
}
