use anyhow::Context;
use itertools::Itertools;
use teloxide::{
    prelude::*,
    types::{
        Chat, ChatKind, InlineKeyboardButton, InlineKeyboardMarkup,
        KeyboardButton, KeyboardMarkup, KeyboardRemove,
    },
};

use crate::{
    cities, text,
    types::{DatingPurpose, Subjects},
    utils, Bot, EditProfile,
};

pub async fn request_set_location_filter(
    bot: &Bot,
    chat: &Chat,
    p: &EditProfile,
) -> anyhow::Result<()> {
    let id = p
        .city
        .context("city must be set at this moment")?
        .context("city must be set at this moment")?;

    let subject =
        cities::subject_by_id(id).context("subject not found")?.to_owned();
    let city = cities::city_by_id(id).context("city not found")?.to_owned();

    let mut subject_city = vec![KeyboardButton::new(subject)];
    if subject != city {
        subject_city.push(KeyboardButton::new(city))
    };

    let keyboard = vec![
        vec![
            KeyboardButton::new("Вся Россия".to_owned()),
            KeyboardButton::new(format!(
                "{} ФО",
                cities::county_by_id(id).context("county not found")?
            )),
        ],
        subject_city,
    ];
    let keyboard_markup = KeyboardMarkup::new(keyboard).resize_keyboard(true);

    bot.send_message(chat.id, text::EDIT_LOCATION_FILTER)
        .reply_markup(keyboard_markup)
        .await?;

    Ok(())
}

pub async fn request_set_city(bot: &Bot, chat: &Chat) -> anyhow::Result<()> {
    let keyboard = vec![vec![KeyboardButton::new("Не указывать")]];
    let keyboard_markup = KeyboardMarkup::new(keyboard).resize_keyboard(true);
    bot.send_message(chat.id, text::REQUEST_CITY)
        .reply_markup(keyboard_markup)
        .await?;
    Ok(())
}

pub async fn request_set_name(bot: &Bot, chat: &Chat) -> anyhow::Result<()> {
    match &chat.kind {
        ChatKind::Public(_) => anyhow::bail!("chat isn't private"),
        ChatKind::Private(p) => match &p.first_name {
            Some(n) => {
                let keyboard = vec![vec![KeyboardButton::new(n)]];
                let keyboard_markup =
                    KeyboardMarkup::new(keyboard).resize_keyboard(true);
                bot.send_message(chat.id, text::REQUEST_NAME)
                    .reply_markup(keyboard_markup)
                    .await?;
                Ok(())
            }
            None => {
                bot.send_message(chat.id, text::REQUEST_NAME).await?;
                Ok(())
            }
        },
    }
}

pub async fn request_set_gender(bot: &Bot, chat: &Chat) -> anyhow::Result<()> {
    let keyboard = vec![vec![
        KeyboardButton::new(text::GENDER_MALE),
        KeyboardButton::new(text::GENDER_FEMALE),
    ]];
    let keyboard_markup = KeyboardMarkup::new(keyboard).resize_keyboard(true);

    bot.send_message(chat.id, text::REQUEST_GENDER)
        .reply_markup(keyboard_markup)
        .await?;
    Ok(())
}

pub async fn request_set_gender_filter(
    bot: &Bot,
    chat: &Chat,
) -> anyhow::Result<()> {
    let keyboard = vec![
        vec![
            KeyboardButton::new(text::GENDER_FILTER_MALE),
            KeyboardButton::new(text::GENDER_FILTER_FEMALE),
        ],
        vec![KeyboardButton::new(text::GENDER_FILTER_ANY)],
    ];
    let keyboard_markup = KeyboardMarkup::new(keyboard).resize_keyboard(true);

    bot.send_message(chat.id, text::REQUEST_GENDER_FILTER)
        .reply_markup(keyboard_markup)
        .await?;
    Ok(())
}

pub async fn request_set_grade(bot: &Bot, chat: &Chat) -> anyhow::Result<()> {
    // let keyboard =
    //     (6..=11).map(|n| KeyboardButton::new(n.to_string())).chunks(3);
    // let keyboard_markup =
    //     KeyboardMarkup::new(keyboard.into_iter()).resize_keyboard(true);

    bot.send_message(chat.id, text::REQUEST_GRADE)
        .reply_markup(KeyboardRemove::new())
        .await?;
    Ok(())
}

pub async fn request_set_subjects(
    bot: &Bot,
    chat: &Chat,
    p: &EditProfile,
) -> anyhow::Result<()> {
    bot.send_message(chat.id, text::EDIT_SUBJECTS)
        .reply_markup(utils::make_subjects_keyboard(
            match p.subjects {
                Some(s) => Subjects::from_bits(s)
                    .context("subjects must be created")?,
                None => Subjects::default(),
            },
            utils::SubjectsKeyboardType::User,
        ))
        .await?;
    Ok(())
}

pub async fn request_set_dating_purpose(
    bot: &Bot,
    chat: &Chat,
    p: &EditProfile,
) -> anyhow::Result<()> {
    bot.send_message(chat.id, text::REQUEST_SET_DATING_PURPOSE)
        .reply_markup(utils::make_dating_purpose_keyboard(
            match p.dating_purpose {
                Some(d) => DatingPurpose::try_from(d)?,
                None => DatingPurpose::default(),
            },
        ))
        .await?;
    Ok(())
}

pub async fn request_set_subjects_filter(
    bot: &Bot,
    chat: &Chat,
    p: &EditProfile,
) -> anyhow::Result<()> {
    bot.send_message(chat.id, text::EDIT_PARTNER_SUBJECTS)
        .reply_markup(utils::make_subjects_keyboard(
            match p.subjects_filter {
                Some(s) => Subjects::from_bits(s)
                    .context("subjects filter must be created")?,
                None => Subjects::default(),
            },
            utils::SubjectsKeyboardType::Partner,
        ))
        .await?;
    Ok(())
}

pub async fn request_set_about(bot: &Bot, chat: &Chat) -> anyhow::Result<()> {
    bot.send_message(chat.id, text::EDIT_ABOUT)
        .reply_markup(KeyboardRemove::new())
        .await?;
    Ok(())
}

pub async fn request_set_photos(bot: &Bot, chat: &Chat) -> anyhow::Result<()> {
    let keyboard = vec![vec![KeyboardButton::new("Без фото")]];
    let keyboard_markup = KeyboardMarkup::new(keyboard).resize_keyboard(true);
    bot.send_message(chat.id, text::REQUEST_SET_PHOTOS)
        .reply_markup(keyboard_markup)
        .await?;
    Ok(())
}

pub async fn request_edit_profile(
    bot: &Bot,
    chat: &Chat,
) -> anyhow::Result<()> {
    let keyboard: Vec<Vec<_>> =
        ["Имя", "Предметы", "О себе", "Город", "Фото", "Отмена"]
            .into_iter()
            .map(|i| InlineKeyboardButton::callback(i, format!("e{i}")))
            .chunks(3)
            .into_iter()
            .map(|row| row.collect())
            .collect();

    bot.send_message(chat.id, text::REQUEST_EDIT)
        .reply_markup(InlineKeyboardMarkup::new(keyboard))
        .await?;
    Ok(())
}
