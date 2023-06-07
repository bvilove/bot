use std::sync::Arc;

use anyhow::{bail, Context};
use db::Database;
use entities::sea_orm_active_enums::{Gender, LocationFilter};
use teloxide::{
    net::Download,
    prelude::*,
    types::{Chat, KeyboardButton, KeyboardMarkup},
};

use crate::{
    cities, db, text, utils, Bot, DatingPurpose, EditProfile, MyDialogue,
    State, Subjects,
};

async fn next_state(
    dialogue: MyDialogue,
    chat: Chat,
    state: State,
    p: EditProfile,
    bot: Bot,
    db: Arc<Database>,
) -> anyhow::Result<()> {
    use State::*;
    let next_state = match state {
        SetName(EditProfile { create_new: true, .. }) => SetGender(p.clone()),
        SetGender(EditProfile { create_new: true, .. }) => {
            SetGenderFilter(p.clone())
        }
        SetGenderFilter(EditProfile { create_new: true, .. }) => {
            SetGraduationYear(p.clone())
        }
        SetGraduationYear(EditProfile { create_new: true, .. }) => {
            SetSubjects(p.clone())
        }
        SetSubjects(EditProfile { create_new: true, .. }) => {
            SetSubjectsFilter(p.clone())
        }
        SetSubjectsFilter(EditProfile { create_new: true, .. }) => {
            SetDatingPurpose(p.clone())
        }
        SetDatingPurpose(EditProfile { create_new: true, .. }) => {
            SetCity(p.clone())
        }
        SetCity(EditProfile { create_new: true, .. }) => {
            SetLocationFilter(p.clone())
        }
        SetLocationFilter(EditProfile { create_new: true, .. }) => {
            SetAbout(p.clone())
        }
        SetAbout(EditProfile { create_new: true, .. }) => {
            db.create_or_update_user(p.clone()).await?;
            SetPhotos(EditProfile::new(chat.id.0))
        }
        Start => {
            dialogue.exit().await?;
            anyhow::bail!("wrong state: {:?}", state)
        }
        _ => {
            db.create_or_update_user(p.clone()).await?;
            crate::datings::send_profile(&bot, &db, p.id).await?;
            Start
        }
    };
    dialogue.update(next_state.clone()).await?;
    print_current_state(&next_state, p, bot, chat).await?;

    Ok(())
}

pub async fn print_current_state(
    state: &State,
    p: EditProfile,
    bot: Bot,
    chat: Chat,
) -> anyhow::Result<()> {
    use State::*;

    use crate::request::*;
    match state {
        SetName(_) => request_set_name(bot, chat).await?,
        SetGender(_) => request_set_gender(bot, chat).await?,
        SetGenderFilter(_) => request_set_gender_filter(bot, chat).await?,
        SetGraduationYear(_) => request_set_graduation_year(bot, chat).await?,
        SetSubjects(_) => request_set_subjects(bot, chat).await?,
        SetSubjectsFilter(_) => request_set_subjects_filter(bot, chat).await?,
        SetDatingPurpose(_) => request_set_dating_purpose(bot, chat).await?,
        SetCity(_) => request_set_city(bot, chat).await?,
        SetLocationFilter(_) => {
            request_set_location_filter(bot, p, chat).await?
        }
        SetAbout(_) => request_set_about(bot, chat).await?,
        SetPhotos(_) => request_set_photos(bot, chat).await?,
        Start => {} // _ => anyhow::bail!("wrong state: {:?}", state),
    };
    Ok(())
}

pub async fn handle_set_city(
    db: Arc<Database>,
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    mut profile: EditProfile,
    state: State,
) -> anyhow::Result<()> {
    let text = msg.text().context("no text in message")?;

    match text {
        "Верно" => {
            if profile.city.is_none() {
                bail!("br moment")
            }
            next_state(dialogue, msg.chat, state, profile, bot, db).await?;
        }
        // "Список городов" => {
        //     let cities: String = crate::cities::cities_list();

        //     bot.send_message(msg.chat.id, cities).await?;
        // }
        _ => match crate::cities::find_city(text) {
            Some(id) => {
                profile.city = Some(id);
                dialogue.update(State::SetCity(profile)).await?;

                let keyboard = vec![vec![
                    KeyboardButton::new("Верно"),
                    KeyboardButton::new("Список городов"),
                ]];
                let keyboard_markup =
                    KeyboardMarkup::new(keyboard).resize_keyboard(true);
                bot.send_message(
                    msg.chat.id,
                    format!("Ваш город - {}?", crate::cities::format_city(id)?),
                )
                .reply_markup(keyboard_markup)
                .await?;
            }
            None => {
                let keyboard =
                    vec![vec![KeyboardButton::new("Список городов")]];
                let keyboard_markup =
                    KeyboardMarkup::new(keyboard).resize_keyboard(true);
                bot.send_message(
                    msg.chat.id,
                    "Не удалось найти город! Попробуйте ещё раз или \
                     посмотрите список доступных.",
                )
                .reply_markup(keyboard_markup)
                .await?;
            }
        },
    }

    Ok(())
}

pub async fn handle_set_partner_city(
    db: Arc<Database>,
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    mut profile: EditProfile,
    state: State,
) -> anyhow::Result<()> {
    let text = msg.text().context("no text in message")?;

    let location_filter = if text == "Вся Россия" {
        LocationFilter::SameCountry
    } else if cities::county_exists(
        &text
            .chars()
            .rev()
            .skip(3)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<String>(),
    ) {
        LocationFilter::SameCounty
    } else if cities::subject_exists(text) {
        LocationFilter::SameSubject
    } else if cities::city_exists(text) {
        LocationFilter::SameCity
    } else {
        print_current_state(&state, profile, bot, msg.chat).await?;
        return Ok(());
    };

    profile.location_filter = Some(location_filter);
    next_state(dialogue, msg.chat, state, profile, bot, db).await?;

    Ok(())
}

pub async fn handle_set_name(
    db: Arc<Database>,
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    mut profile: EditProfile,
    state: State,
) -> anyhow::Result<()> {
    match msg.text() {
        Some(text) if (3..=16).contains(&text.chars().count()) => {
            profile.name = Some(text.to_owned());
            next_state(dialogue, msg.chat, state, profile, bot, db).await?;
        }
        _ => {
            print_current_state(&state, profile, bot, msg.chat).await?;
        }
    }
    Ok(())
}

pub async fn handle_set_gender(
    db: Arc<Database>,
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    mut profile: EditProfile,
    state: State,
) -> anyhow::Result<()> {
    let gender = match msg.text().context("no text in message")? {
        text::GENDER_MALE => Gender::Male,
        text::GENDER_FEMALE => Gender::Female,
        _ => {
            print_current_state(&state, profile, bot, msg.chat).await?;
            return Ok(());
        }
    };

    profile.gender = Some(gender);
    next_state(dialogue, msg.chat, state, profile, bot, db).await?;

    Ok(())
}

pub async fn handle_set_partner_gender(
    db: Arc<Database>,
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    mut profile: EditProfile,
    state: State,
) -> anyhow::Result<()> {
    let gender = match msg.text().context("no text in message")? {
        text::GENDER_FILTER_MALE => Some(Gender::Male),
        text::GENDER_FILTER_FEMALE => Some(Gender::Female),
        text::GENDER_FILTER_ANY => None,
        _ => {
            print_current_state(&state, profile, bot, msg.chat).await?;
            return Ok(());
        }
    };

    profile.gender_filter = Some(gender);
    next_state(dialogue, msg.chat, state, profile, bot, db).await?;

    Ok(())
}

pub async fn handle_set_graduation_year(
    db: Arc<Database>,
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    mut profile: EditProfile,
    state: State,
) -> anyhow::Result<()> {
    let Ok(grade) = msg
        .text()
        .context("no text in message")?
        .parse::<i32>()
    else {
        print_current_state(&state, profile, bot, msg.chat).await?;
        return Ok(())
    };

    let graduation_year = utils::graduation_year_from_grade(grade)?;

    profile.graduation_year = Some(graduation_year as i16);
    next_state(dialogue, msg.chat, state, profile, bot, db).await?;

    // bot.send_message(
    //     msg.chat.id,
    //     format!(
    //         "Хорошо, сейчас вы в {grade} классе и закончите школу в \
    //          {graduation_year} году.\nИзменить это можно командой /setgrade"
    //     ),
    // )
    // .reply_markup(KeyboardRemove::new())
    // .await?;
    // print_next_state(&state, bot, msg.chat).await?;

    Ok(())
}

pub async fn handle_set_subjects_callback(
    db: Arc<Database>,
    bot: Bot,
    dialogue: MyDialogue,
    mut profile: EditProfile,
    state: State,
    q: CallbackQuery,
) -> anyhow::Result<()> {
    let text = q.data.context("callback data not provided")?;
    let msg = q.message.context("callback without message")?;

    let subjects = match profile.subjects {
        Some(s) => {
            Subjects::from_bits(s).context("subjects must be created")?
        }
        None => Subjects::empty(),
    };

    if text == "continue" {
        bot.edit_message_reply_markup(msg.chat.id, msg.id).await?;

        let subjects_str = if subjects.is_empty() {
            "Вы ничего не ботаете.".to_owned()
        } else {
            format!(
                "Предметы, которые вы ботаете: {}.",
                utils::subjects_list(subjects)?
            )
        };
        bot.edit_message_text(
            msg.chat.id,
            msg.id,
            format!(
                "{subjects_str}\nЧтобы изменить предметы, которые вы \
                 ботаете, используйте команду /setsubjects",
            ),
        )
        .await?;

        profile.subjects = Some(subjects.bits());
        next_state(dialogue, msg.chat, state, profile, bot, db).await?;
    } else {
        let subjects = subjects
            ^ Subjects::from_bits(text.parse()?).context("subjects error")?;

        bot.edit_message_reply_markup(msg.chat.id, msg.id)
            .reply_markup(utils::make_subjects_keyboard(
                subjects,
                utils::SubjectsKeyboardType::User,
            ))
            .await?;

        profile.subjects = Some(subjects.bits());
        dialogue.update(State::SetSubjects(profile)).await?;
    }
    Ok(())
}

pub async fn handle_set_subjects_filter_callback(
    db: Arc<Database>,
    bot: Bot,
    dialogue: MyDialogue,
    mut profile: EditProfile,
    state: State,
    q: CallbackQuery,
) -> anyhow::Result<()> {
    let text = q.data.context("callback data not provided")?;
    let msg = q.message.context("callback without message")?;

    let subjects_filter = match profile.subjects_filter {
        Some(s) => {
            Subjects::from_bits(s).context("subjects must be created")?
        }
        None => Subjects::empty(),
    };

    if text == "continue" {
        bot.edit_message_reply_markup(msg.chat.id, msg.id).await?;

        let subjects_filter_str = if subjects_filter.is_empty() {
            "Не важно, что ботает другой человек.".to_owned()
        } else {
            format!(
                "Предметы, хотя бы один из которых должен ботать тот, кого вы \
                 ищете: {}.",
                utils::subjects_list(subjects_filter)?
            )
        };
        bot.edit_message_text(
            msg.chat.id,
            msg.id,
            format!(
                "{subjects_filter_str}\nЧтобы изменить их, используйте \
                 /filtersubjects",
            ),
        )
        .await?;

        profile.subjects_filter = Some(subjects_filter.bits());
        next_state(dialogue, msg.chat, state, profile, bot, db).await?;
    } else {
        let subjects_filter = subjects_filter
            ^ Subjects::from_bits(text.parse()?).context("subjects error")?;

        bot.edit_message_reply_markup(msg.chat.id, msg.id)
            .reply_markup(utils::make_subjects_keyboard(
                subjects_filter,
                utils::SubjectsKeyboardType::User,
            ))
            .await?;

        profile.subjects_filter = Some(subjects_filter.bits());
        dialogue.update(State::SetSubjects(profile)).await?;
    }
    Ok(())
}

pub async fn handle_set_dating_purpose_callback(
    db: Arc<Database>,
    bot: Bot,
    dialogue: MyDialogue,
    mut profile: EditProfile,
    state: State,
    q: CallbackQuery,
) -> anyhow::Result<()> {
    let text = q.data.context("callback data not provided")?;
    let msg = q.message.context("callback without message")?;

    let purpose = match profile.dating_purpose {
        Some(s) => {
            DatingPurpose::from_bits(s).context("purpose must be created")?
        }
        None => DatingPurpose::empty(),
    };

    if text == "continue" {
        if purpose == DatingPurpose::empty() {
            bail!("there must be at least 1 purpose")
        }

        bot.edit_message_reply_markup(msg.chat.id, msg.id).await?;

        bot.edit_message_text(
            msg.chat.id,
            msg.id,
            format!(
                "Вас интересует: {}.",
                utils::dating_purpose_list(purpose)?
            ),
        )
        .await?;

        profile.dating_purpose = Some(purpose.bits());
        next_state(dialogue, msg.chat, state, profile, bot, db).await?;
    } else {
        let purpose = purpose
            ^ DatingPurpose::from_bits(text.parse()?)
                .context("purpose error")?;

        bot.edit_message_reply_markup(msg.chat.id, msg.id)
            .reply_markup(utils::make_dating_purpose_keyboard(purpose))
            .await?;

        profile.dating_purpose = Some(purpose.bits());
        dialogue.update(State::SetDatingPurpose(profile)).await?;
    }
    Ok(())
}

pub async fn handle_set_about(
    db: Arc<Database>,
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    mut profile: EditProfile,
    state: State,
) -> anyhow::Result<()> {
    match msg.text() {
        Some(text) if (1..=1024).contains(&text.chars().count()) => {
            profile.about = Some(text.to_owned());
            next_state(dialogue, msg.chat, state, profile, bot, db).await?;
        }
        _ => {
            print_current_state(&state, profile, bot, msg.chat).await?;
        }
    }
    Ok(())
}

pub async fn handle_set_photos(
    db: Arc<Database>,
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    mut profile: EditProfile,
    state: State,
) -> anyhow::Result<()> {
    let Some(photo_sizes) = msg.photo() else {
        match msg.text() {
            Some(text) if text == "Без фото" => {
                db.clean_images(msg.chat.id.0).await?;
                next_state(dialogue, msg.chat, state, profile, bot, db).await?;
            },
            Some(text) if text == "Сохранить фото" => {
                next_state(dialogue, msg.chat, state, profile, bot, db).await?;
            },
            _ => {
                print_current_state(&state, profile, bot, msg.chat).await?;
            }
        };
        return Ok(())
    };

    let keyboard = vec![vec![KeyboardButton::new("Сохранить фото")]];
    let keyboard_markup = KeyboardMarkup::new(keyboard).resize_keyboard(true);

    if profile.photos_count == 0 {
        db.clean_images(msg.chat.id.0).await?;
    } else if profile.photos_count >= 9 {
        bot.send_message(msg.chat.id, "Невозможно добавить более 9 фото")
            .reply_markup(keyboard_markup)
            .await?;
        return Ok(());
    };

    profile.photos_count += 1;

    let photo = &photo_sizes[photo_sizes.len() - 1];
    let photo_file = bot.get_file(photo.file.clone().id).await?;

    let mut photo_buf = vec![0u8; photo_file.size as usize];
    bot.download_file(&photo_file.path, &mut photo_buf).await?;

    db.create_image(msg.chat.id.0, photo_file.id.clone(), photo_buf).await?;

    bot.send_message(
        msg.chat.id,
        format!("Добавлено {}/9 фото. Добавить ещё?", profile.photos_count),
    )
    .reply_markup(keyboard_markup)
    .await?;

    dialogue.update(State::SetPhotos(profile)).await?;

    Ok(())
}
