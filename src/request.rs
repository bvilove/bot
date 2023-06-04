use itertools::Itertools;
use teloxide::{
    prelude::*,
    types::{Chat, ChatKind, KeyboardButton, KeyboardMarkup},
};

use crate::{text, utils, Bot, Subjects};

pub async fn request_set_city(bot: Bot, chat: Chat) -> anyhow::Result<()> {
    let keyboard = vec![vec![
        KeyboardButton::new(text::USER_CITY_CURRENT),
        KeyboardButton::new(text::USER_CITY_ANY),
    ]];
    let keyboard_markup = KeyboardMarkup::new(keyboard).resize_keyboard(true);

    bot.send_message(chat.id, text::EDIT_CITY)
        .reply_markup(keyboard_markup)
        .await?;
    Ok(())
}

pub async fn request_set_partner_city(
    bot: Bot,
    chat: Chat,
) -> anyhow::Result<()> {
    // TODO
    // let keyboard = vec![vec![
    //     KeyboardButton::new(text::USER_CITY_CURRENT),
    //     KeyboardButton::new(text::USER_CITY_ANY),
    // ]];
    // let keyboard_markup = KeyboardMarkup::new(keyboard).resize_keyboard(true);

    // bot.send_message(chat.id, text::EDIT_CITY)
    //     .reply_markup(keyboard_markup)
    //     .await?;
    Ok(())
}

pub async fn request_set_name(bot: Bot, chat: Chat) -> anyhow::Result<()> {
    match chat.kind {
        ChatKind::Public(_) => anyhow::bail!("chat isn't private"),
        ChatKind::Private(p) => match p.first_name {
            Some(n) => {
                let keyboard = vec![vec![KeyboardButton::new(n)]];
                let keyboard_markup =
                    KeyboardMarkup::new(keyboard).resize_keyboard(true);
                bot.send_message(chat.id, text::EDIT_NAME)
                    .reply_markup(keyboard_markup)
                    .await?;
                Ok(())
            }
            None => {
                bot.send_message(chat.id, text::EDIT_NAME).await?;
                Ok(())
            }
        },
    }
}

pub async fn request_set_gender(bot: Bot, chat: Chat) -> anyhow::Result<()> {
    let keyboard = vec![vec![
        KeyboardButton::new(text::USER_GENDER_MALE),
        KeyboardButton::new(text::USER_GENDER_FEMALE),
    ]];
    let keyboard_markup = KeyboardMarkup::new(keyboard).resize_keyboard(true);

    bot.send_message(chat.id, text::EDIT_GENDER)
        .reply_markup(keyboard_markup)
        .await?;
    Ok(())
}

pub async fn request_set_partner_gender(
    bot: Bot,
    chat: Chat,
) -> anyhow::Result<()> {
    let keyboard = vec![
        vec![
            KeyboardButton::new(text::PARTNER_GENDER_MALE),
            KeyboardButton::new(text::PARTNER_GENDER_FEMALE),
        ],
        vec![KeyboardButton::new(text::PARTNER_GENDER_ALL)],
    ];
    let keyboard_markup = KeyboardMarkup::new(keyboard).resize_keyboard(true);

    bot.send_message(chat.id, text::EDIT_PARTNER_GENDER)
        .reply_markup(keyboard_markup)
        .await?;
    Ok(())
}

pub async fn request_set_graduation_year(
    bot: Bot,
    chat: Chat,
) -> anyhow::Result<()> {
    let keyboard =
        (6..=11).map(|n| KeyboardButton::new(n.to_string())).chunks(3);
    let keyboard_markup =
        KeyboardMarkup::new(keyboard.into_iter()).resize_keyboard(true);

    bot.send_message(chat.id, text::REQUEST_GRADE)
        .reply_markup(keyboard_markup)
        .await?;
    Ok(())
}

pub async fn request_set_subjects(bot: Bot, chat: Chat) -> anyhow::Result<()> {
    bot.send_message(chat.id, text::EDIT_SUBJECTS)
        .reply_markup(utils::make_subjects_keyboard(
            Subjects::default(),
            utils::SubjectsKeyboardType::User,
        ))
        .await?;
    Ok(())
}

pub async fn request_set_partner_subjects(
    bot: Bot,
    chat: Chat,
) -> anyhow::Result<()> {
    bot.send_message(chat.id, text::EDIT_PARTNER_SUBJECTS)
        .reply_markup(utils::make_subjects_keyboard(
            Subjects::default(),
            utils::SubjectsKeyboardType::Partner,
        ))
        .await?;
    Ok(())
}

pub async fn request_set_about(bot: Bot, chat: Chat) -> anyhow::Result<()> {
    bot.send_message(chat.id, text::EDIT_ABOUT).await?;
    Ok(())
}
