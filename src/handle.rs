use std::sync::Arc;

use anyhow::{bail, Context};
use db::Database;
use entities::sea_orm_active_enums::Gender;
use teloxide::{
    net::Download,
    prelude::*,
    types::{Chat, KeyboardButton, KeyboardMarkup},
};

use crate::{
    db, text, utils, Bot, EditProfile, MyDialogue, Profile, State, Subjects,
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
        SetName(EditProfile { create_new: true, .. }) => SetGender(p),
        SetGender(EditProfile { create_new: true, .. }) => SetPartnerGender(p),
        SetPartnerGender(EditProfile { create_new: true, .. }) => {
            SetGraduationYear(p)
        }
        SetGraduationYear(EditProfile { create_new: true, .. }) => {
            SetSubjects(p)
        }
        SetSubjects(EditProfile { create_new: true, .. }) => {
            SetPartnerSubjects(p)
        }
        SetPartnerSubjects(EditProfile { create_new: true, .. }) => SetCity(p),
        SetCity(EditProfile { create_new: true, .. }) => SetPartnerCity(p),
        SetPartnerCity(EditProfile { create_new: true, .. }) => SetAbout(p),
        SetAbout(EditProfile { create_new: true, .. }) => {
            let profile = Profile::try_from(p.clone())?;
            db.create_user(
                dialogue.chat_id().0,
                profile.name,
                profile.about,
                profile.gender,
                profile.partner_gender,
                profile.graduation_year,
                profile.subjects.0 .0,
                profile.partner_subjects.bits(),
                profile.city,
                profile.same_partner_city,
            )
            .await?;

            SetPhotos(p)
        }
        Start => {
            dialogue.exit().await?;
            anyhow::bail!("wrong state: {:?}", state)
        }
        _ => {
            // TODO: call db
            Start
        }
    };
    print_current_state(&next_state, bot, chat).await?;

    dialogue.update(next_state).await?;
    Ok(())
}

pub async fn print_current_state(
    state: &State,
    bot: Bot,
    chat: Chat,
) -> anyhow::Result<()> {
    use State::*;

    use crate::request::*;
    match state {
        SetName(_) => request_set_name(bot, chat).await?,
        SetGender(_) => request_set_gender(bot, chat).await?,
        SetPartnerGender(_) => request_set_partner_gender(bot, chat).await?,
        SetGraduationYear(_) => request_set_graduation_year(bot, chat).await?,
        SetSubjects(_) => request_set_subjects(bot, chat).await?,
        SetPartnerSubjects(_) => {
            request_set_partner_subjects(bot, chat).await?
        }
        SetCity(_) => request_set_city(bot, chat).await?,
        SetPartnerCity(_) => request_set_partner_city(bot, chat).await?,
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
        "Список городов" => {
            let cities: String = crate::cities::cities_list();

            bot.send_message(msg.chat.id, cities).await?;
        }
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
    let same_partner_city = match msg.text().context("no text in message")? {
        text::USER_CITY_CURRENT => true,
        text::USER_CITY_ANY => false,
        _ => {
            print_current_state(&state, bot, msg.chat).await?;
            return Ok(());
        }
    };

    profile.same_partner_city = Some(same_partner_city);
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
        Some(text) if (3..=30).contains(&text.len()) => {
            profile.name = Some(text.to_owned());
            next_state(dialogue, msg.chat, state, profile, bot, db).await?;

            // bot.send_message(
            //     msg.chat.id,
            //     format!(
            //         "Выбранное имя: {text}.\nЕго можно будет изменить позже \
            //          командой /setname"
            //     ),
            // )
            // .await?;
            // print_next_state(&state, bot, msg.chat).await?;
        }
        _ => {
            print_current_state(&state, bot, msg.chat).await?;
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
        text::USER_GENDER_MALE => Gender::Male,
        text::USER_GENDER_FEMALE => Gender::Female,
        _ => {
            print_current_state(&state, bot, msg.chat).await?;
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
        text::PARTNER_GENDER_MALE => Some(Gender::Male),
        text::PARTNER_GENDER_FEMALE => Some(Gender::Female),
        text::PARTNER_GENDER_ALL => None,
        _ => {
            print_current_state(&state, bot, msg.chat).await?;
            return Ok(());
        }
    };

    profile.partner_gender = Some(gender);
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
        print_current_state(&state, bot, msg.chat).await?;
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

    if text == text::SUBJECTS_CONTINUE || text == text::SUBJECTS_USER_EMPTY {
        profile.subjects = Some(profile.subjects.unwrap_or_default());

        bot.edit_message_reply_markup(msg.chat.id, msg.id).await?;

        let user_subjects =
            if profile.subjects.context("subjects must be set")?.is_empty() {
                "Вы ничего не ботаете.".to_owned()
            } else {
                format!(
                    "Предметы, которые вы ботаете: {}.",
                    utils::subjects_list(
                        profile.subjects.context("subjects must be set")?,
                    )?
                )
            };
        bot.edit_message_text(
            msg.chat.id,
            msg.id,
            format!(
                "{user_subjects}\nЧтобы изменить предметы, которые вы \
                 ботаете, используйте команду /setsubjects",
            ),
        )
        .await?;

        next_state(dialogue, msg.chat, state, profile, bot, db).await?;
        // print_next_state(&state, bot, msg.chat).await?;
    } else {
        let subjects = profile.subjects.unwrap_or_default()
            ^ Subjects::from_bits(text.parse()?).context("subjects error")?;
        profile.subjects = Some(subjects);
        bot.edit_message_reply_markup(msg.chat.id, msg.id)
            .reply_markup(utils::make_subjects_keyboard(
                subjects,
                utils::SubjectsKeyboardType::User,
            ))
            .await?;
        dialogue.update(State::SetSubjects(profile)).await?;
    }
    Ok(())
}

pub async fn handle_set_partner_subjects_callback(
    db: Arc<Database>,
    bot: Bot,
    dialogue: MyDialogue,
    mut profile: EditProfile,
    state: State,
    q: CallbackQuery,
) -> anyhow::Result<()> {
    let text = q.data.context("callback data not provided")?;
    let msg = q.message.context("callback without message")?;

    if text == text::SUBJECTS_CONTINUE || text == text::SUBJECTS_PARTNER_EMPTY {
        profile.partner_subjects =
            Some(profile.partner_subjects.unwrap_or_default());

        bot.edit_message_reply_markup(msg.chat.id, msg.id).await?;

        let partner_subjects = if profile
            .partner_subjects
            .context("subjects must be set")?
            .is_empty()
        {
            "Не важно, что ботает другой человек.".to_owned()
        } else {
            format!(
                "Предметы, хотя бы один из которых должен ботать тот, кого вы \
                 ищете: {}.",
                utils::subjects_list(
                    profile.partner_subjects.context("subjects must be set")?,
                )?
            )
        };
        bot.edit_message_text(
            msg.chat.id,
            msg.id,
            format!(
                "{partner_subjects}\nЧтобы изменить их, используйте \
                 /filtersubjects",
            ),
        )
        .await?;

        next_state(dialogue, msg.chat, state, profile, bot, db).await?;
        // print_next_state(&state, bot, msg.chat).await?;
    } else {
        let subjects = profile.partner_subjects.unwrap_or_default()
            ^ Subjects::from_bits(text.parse()?).context("subjects error")?;
        profile.partner_subjects = Some(subjects);
        bot.edit_message_reply_markup(msg.chat.id, msg.id)
            .reply_markup(utils::make_subjects_keyboard(
                subjects,
                utils::SubjectsKeyboardType::Partner,
            ))
            .await?;
        dialogue.update(State::SetPartnerSubjects(profile)).await?;
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
        Some(text) if (1..=1000).contains(&text.len()) => {
            profile.about = Some(text.to_owned());

            next_state(dialogue, msg.chat, state, profile, bot, db).await?;
            // print_next_state(&state, bot, msg.chat).await?;
        }
        _ => {
            print_current_state(&state, bot, msg.chat).await?;
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
    let Some(photo_sizes) = msg.photo() else {match msg.text() {
        Some(text) if text == "Без фото" => {
            db.clean_images(msg.chat.id.0).await?;
            next_state(dialogue, msg.chat, state, profile, bot, db).await?;
        },
        Some(text) if text == "Сохранить фото" => {
            next_state(dialogue, msg.chat, state, profile, bot, db).await?;
        },
        _ => {
            print_current_state(&state, bot, msg.chat).await?;
        }
    };
    return Ok(())};

    if profile.photos_count == 0 {
        db.clean_images(msg.chat.id.0).await?;
    };

    profile.photos_count += 1;

    let photo = &photo_sizes[photo_sizes.len() - 1];
    let photo_file = bot.get_file(photo.file.clone().id).await?;

    let mut photo_buf = vec![0u8; photo_file.size as usize];
    bot.download_file(&photo_file.path, &mut photo_buf).await?;

    db.create_image(msg.chat.id.0, photo_file.id.clone(), photo_buf).await?;

    let keyboard = vec![vec![KeyboardButton::new("Сохранить фото")]];
    let keyboard_markup = KeyboardMarkup::new(keyboard).resize_keyboard(true);
    bot.send_message(
        msg.chat.id,
        format!("Добавлено {} фото. Добавить ещё?", profile.photos_count),
    )
    .reply_markup(keyboard_markup)
    .await?;

    dialogue.update(State::SetPhotos(profile)).await?;

    Ok(())
}
