use std::sync::Arc;

use anyhow::{bail, Context};
use db::Database;
use entities::sea_orm_active_enums::{Gender, ImageKind, LocationFilter};
use teloxide::{
    // net::Download,
    prelude::*,
    types::{
        Chat, InlineKeyboardButton, InlineKeyboardMarkup, KeyboardButton,
        KeyboardMarkup, KeyboardRemove,
    },
};
use tracing::instrument;

use crate::{
    cities::{self, City},
    db, text,
    types::{DatingPurpose, Subjects, GraduationYear, Grade},
    utils, Bot, EditProfile, MyDialogue, State,
};

#[instrument(level = "debug", skip(bot, db))]
pub async fn next_state(
    dialogue: &MyDialogue,
    chat: &Chat,
    state: &State,
    p: EditProfile,
    bot: &Bot,
    db: &Arc<Database>,
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
        SetSubjects(_) => SetSubjectsFilter(p.clone()),
        SetSubjectsFilter(EditProfile { create_new: true, .. }) => {
            SetDatingPurpose(p.clone())
        }
        SetDatingPurpose(EditProfile { create_new: true, .. }) => {
            SetCity(p.clone())
        }
        SetCity(_) => {
            if p.city
                .context("city must be set when city editing finished")?
                .is_some()
            {
                SetLocationFilter(p.clone())
            } else if p.create_new {
                SetAbout(p.clone())
            } else {
                Start
            }
        }
        SetLocationFilter(EditProfile { create_new: true, .. }) => {
            SetAbout(p.clone())
        }
        SetAbout(EditProfile { create_new: true, .. }) => {
            // HACK: create user before setting photos
            db.create_or_update_user(p.clone()).await?;
            SetPhotos(p.clone())
        }
        SetPhotos(_) => {
            crate::datings::send_profile(bot, db, p.id).await?;
            Start
        }
        // invalid states
        Start | LikeMessage { .. } | Edit => {
            dialogue.exit().await?;
            anyhow::bail!("wrong state: {:?}", state)
        }
        // *(EditProfile { create_new: true, .. })
        _ => {
            db.create_or_update_user(p.clone()).await?;
            crate::datings::send_profile(bot, db, p.id).await?;
            Start
        }
    };
    print_current_state(&next_state, Some(&p), bot, chat).await?;
    dialogue.update(next_state).await?;

    Ok(())
}

#[instrument(level = "debug", skip(bot))]
pub async fn print_current_state(
    state: &State,
    p: Option<&EditProfile>,
    bot: &Bot,
    chat: &Chat,
) -> anyhow::Result<()> {
    use State::*;

    use crate::request::*;

    match state {
        // edit profile
        SetName(_) => request_set_name(bot, chat).await?,
        SetGender(_) => request_set_gender(bot, chat).await?,
        SetGenderFilter(_) => request_set_gender_filter(bot, chat).await?,
        SetGraduationYear(_) => request_set_grade(bot, chat).await?,
        SetSubjects(_) => {
            request_set_subjects(
                bot,
                chat,
                p.context("profile must be provided")?,
            )
            .await?
        }
        SetSubjectsFilter(_) => {
            request_set_subjects_filter(
                bot,
                chat,
                p.context("profile must be provided")?,
            )
            .await?
        }
        SetDatingPurpose(_) => {
            request_set_dating_purpose(
                bot,
                chat,
                p.context("profile must be provided")?,
            )
            .await?
        }
        SetCity(_) => request_set_city(bot, chat).await?,
        SetLocationFilter(_) => {
            request_set_location_filter(
                bot,
                chat,
                p.context("profile must be provided")?,
            )
            .await?
        }
        SetAbout(_) => request_set_about(bot, chat).await?,
        SetPhotos(_) => request_set_photos(bot, chat).await?,
        // others
        LikeMessage { .. } => {
            crate::datings::request_like_msg(bot, chat).await?
        }
        Edit => request_edit_profile(bot, chat).await?,
        // invalid states
        Start => {}
    };
    Ok(())
}

#[instrument(level = "debug", skip(bot, db))]
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
                bail!("try to confirm not set city")
            }
            next_state(&dialogue, &msg.chat, &state, profile, &bot, &db)
                .await?;
        }
        "Не указывать" => {
            profile.city = Some(None);
            profile.location_filter = Some(LocationFilter::SameCountry);

            bot.send_message(msg.chat.id, text::NO_CITY)
                .reply_markup(KeyboardRemove::new())
                .await?;

            next_state(&dialogue, &msg.chat, &state, profile, &bot, &db)
                .await?;
        }
        // "Список городов" => {
        //     let cities: String = crate::cities::cities_list();

        //     bot.send_message(msg.chat.id, cities).await?;
        // }
        _ => match text.parse::<City>() {
            Ok(city) => {
                profile.city = Some(city.into());
                dialogue.update(State::SetCity(profile)).await?;

                let keyboard = vec![vec![
                    KeyboardButton::new("Верно"),
                    KeyboardButton::new("Не указывать"),
                ]];
                let keyboard_markup =
                    KeyboardMarkup::new(keyboard).resize_keyboard(true);
                bot.send_message(msg.chat.id, format!("Ваш город - {city}?",))
                    .reply_markup(keyboard_markup)
                    .await?;
            }
            Err(_) => {
                let keyboard = vec![vec![KeyboardButton::new("Не указывать")]];
                let keyboard_markup =
                    KeyboardMarkup::new(keyboard).resize_keyboard(true);
                bot.send_message(msg.chat.id, text::CANT_FIND_CITY)
                    .reply_markup(keyboard_markup)
                    .await?;
            }
        },
    }

    Ok(())
}

#[instrument(level = "debug", skip(bot, db))]
pub async fn handle_set_location_filter(
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
        print_current_state(&state, Some(&profile), &bot, &msg.chat).await?;
        return Ok(());
    };

    profile.location_filter = Some(location_filter);
    next_state(&dialogue, &msg.chat, &state, profile, &bot, &db).await?;

    Ok(())
}

#[instrument(level = "debug", skip(bot, db))]
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
            next_state(&dialogue, &msg.chat, &state, profile, &bot, &db)
                .await?;
        }
        _ => {
            print_current_state(&state, Some(&profile), &bot, &msg.chat)
                .await?;
        }
    }
    Ok(())
}

#[instrument(level = "debug", skip(bot, db))]
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
            print_current_state(&state, Some(&profile), &bot, &msg.chat)
                .await?;
            return Ok(());
        }
    };

    profile.gender = Some(gender);
    next_state(&dialogue, &msg.chat, &state, profile, &bot, &db).await?;

    Ok(())
}

#[instrument(level = "debug", skip(bot, db))]
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
            print_current_state(&state, Some(&profile), &bot, &msg.chat)
                .await?;
            return Ok(());
        }
    };

    profile.gender_filter = Some(gender);
    next_state(&dialogue, &msg.chat, &state, profile, &bot, &db).await?;

    Ok(())
}

#[instrument(level = "debug", skip(bot, db))]
pub async fn handle_set_grade(
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
        .parse::<i8>()
    else {
        print_current_state(&state, Some(&profile), &bot, &msg.chat).await?;
        return Ok(())
    };

    let Ok(grade) = Grade::try_from(grade) else {
        print_current_state(&state, Some(&profile), &bot, &msg.chat).await?;
        return Ok(());
    };

    let graduation_year: GraduationYear = grade.into();

    profile.graduation_year = Some(graduation_year.into());
    next_state(&dialogue, &msg.chat, &state, profile, &bot, &db).await?;

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

#[instrument(level = "debug", skip(bot, db))]
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
            next_state(&dialogue, &msg.chat, &state, profile, &bot, &db)
                .await?;
        }
        _ => {
            print_current_state(&state, Some(&profile), &bot, &msg.chat)
                .await?;
        }
    }
    Ok(())
}

#[instrument(level = "debug", skip(bot, db))]
pub async fn handle_set_photos(
    db: Arc<Database>,
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    mut profile: EditProfile,
    state: State,
) -> anyhow::Result<()> {
    let keyboard = vec![vec![KeyboardButton::new("Сохранить")]];
    let keyboard_markup = KeyboardMarkup::new(keyboard).resize_keyboard(true);

    match msg.text() {
        Some(text) if text == "Без фото" => {
            db.clean_images(msg.chat.id.0).await?;
            next_state(&dialogue, &msg.chat, &state, profile, &bot, &db)
                .await?;
            return Ok(());
        }
        Some(text) if text == "Сохранить" => {
            next_state(&dialogue, &msg.chat, &state, profile, &bot, &db)
                .await?;
            return Ok(());
        }
        _ => {
            if profile.photos_count == 0 {
                db.clean_images(msg.chat.id.0).await?;
            } else if profile.photos_count >= 10 {
                bot.send_message(
                    msg.chat.id,
                    "Невозможно добавить более 10 фото/видео",
                )
                .reply_markup(keyboard_markup)
                .await?;
                return Ok(());
            };

            if let Some(photo_sizes) = msg.photo() {
                let photo = &photo_sizes[photo_sizes.len() - 1];
                let photo_file = bot.get_file(photo.file.clone().id).await?;

                db.create_image(
                    msg.chat.id.0,
                    photo_file.id.clone(),
                    ImageKind::Image,
                )
                .await?;
            } else if let Some(video) = msg.video() {
                let video_file = bot.get_file(video.file.clone().id).await?;

                db.create_image(
                    msg.chat.id.0,
                    video_file.id.clone(),
                    ImageKind::Video,
                )
                .await?;
            } else {
                print_current_state(&state, Some(&profile), &bot, &msg.chat)
                    .await?;
            };
        }
    };

    profile.photos_count += 1;

    bot.send_message(
        msg.chat.id,
        format!(
            "Добавлено {}/10 фото/видео. Добавить ещё?",
            profile.photos_count
        ),
    )
    .reply_markup(keyboard_markup)
    .await?;

    dialogue.update(State::SetPhotos(profile)).await?;

    Ok(())
}

#[instrument(level = "debug", skip(bot, db))]
pub async fn handle_callback(
    bot: Bot,
    db: Arc<Database>,
    dialogue: MyDialogue,
    state: State,
    q: CallbackQuery,
) -> anyhow::Result<()> {
    let data = q.data.context("callback data not provided")?;
    let msg = q.message.context("callback without message")?;

    let first_char = data.chars().next().context("first char not found")?;
    let last_chars = data.chars().skip(1).collect::<String>();

    fn get_profile(state: &State) -> anyhow::Result<EditProfile> {
        match state {
            State::SetSubjects(e) => Ok(e.clone()),
            State::SetSubjectsFilter(e) => Ok(e.clone()),
            State::SetDatingPurpose(e) => Ok(e.clone()),
            _ => bail!("failed to get profile from state"),
        }
    }

    match first_char {
        // Start profile creation
        '✍' => {
            bot.edit_message_reply_markup(msg.chat.id, msg.id).await?;
            crate::start_profile_creation(&dialogue, &msg, &bot).await?;
        }
        // Find partner
        '🚀' => {
            bot.edit_message_reply_markup(msg.chat.id, msg.id).await?;
            crate::datings::send_recommendation(&bot, &db, msg.chat.id).await?;
        }
        // Edit profile
        'e' => {
            use State::*;

            let user =
                db.get_user(msg.chat.id.0).await?.context("user not found")?;
            let p = EditProfile::from_model(user);
            let state = match last_chars.as_str() {
                "Имя" => SetName(p.clone()),
                "Предметы" => SetSubjects(p.clone()),
                "О себе" => SetAbout(p.clone()),
                "Город" => SetCity(p.clone()),
                "Фото" => SetPhotos(p.clone()),
                "Отмена" => {
                    bot.edit_message_reply_markup(msg.chat.id, msg.id).await?;
                    dialogue.exit().await?;
                    return Ok(());
                }
                _ => {
                    bot.edit_message_reply_markup(msg.chat.id, msg.id).await?;
                    crate::request::request_edit_profile(&bot, &msg.chat)
                        .await?;
                    return Ok(());
                }
            };

            bot.edit_message_reply_markup(msg.chat.id, msg.id).await?;
            print_current_state(&state, Some(&p), &bot, &msg.chat).await?;
            dialogue.update(state).await?;
        }
        // Dating purpose
        'p' => {
            let mut profile = get_profile(&state)?;

            let purpose = match profile.dating_purpose {
                Some(s) => DatingPurpose::try_from(s)?,
                None => DatingPurpose::empty(),
            };

            if last_chars == "continue" {
                if purpose == DatingPurpose::empty() {
                    bail!("there must be at least 1 purpose")
                }

                bot.edit_message_reply_markup(msg.chat.id, msg.id).await?;

                bot.edit_message_text(
                    msg.chat.id,
                    msg.id,
                    format!("Вас интересует: {purpose}.",),
                )
                .await?;

                profile.dating_purpose = Some(purpose.bits());
                next_state(&dialogue, &msg.chat, &state, profile, &bot, &db)
                    .await?;
            } else {
                let purpose = purpose
                    ^ DatingPurpose::from_bits(last_chars.parse()?)
                        .context("purpose error")?;

                bot.edit_message_reply_markup(msg.chat.id, msg.id)
                    .reply_markup(utils::make_dating_purpose_keyboard(purpose))
                    .await?;

                profile.dating_purpose = Some(purpose.bits());
                dialogue.update(State::SetDatingPurpose(profile)).await?;
            }
        }
        // Subjects
        's' => {
            let mut profile = get_profile(&state)?;

            let subjects = match profile.subjects {
                Some(s) => Subjects::from_bits(s)
                    .context("subjects must be created")?,
                None => Subjects::empty(),
            };

            if last_chars == "continue" {
                bot.edit_message_reply_markup(msg.chat.id, msg.id).await?;

                let subjects_str = if subjects.is_empty() {
                    "Вы ничего не ботаете.".to_owned()
                } else {
                    format!("Предметы, которые вы ботаете: {subjects}.",)
                };
                bot.edit_message_text(
                    msg.chat.id,
                    msg.id,
                    format!("{subjects_str}",),
                )
                .await?;

                profile.subjects = Some(subjects.bits());
                next_state(&dialogue, &msg.chat, &state, profile, &bot, &db)
                    .await?;
            } else {
                let subjects = subjects
                    ^ Subjects::from_bits(last_chars.parse()?)
                        .context("subjects error")?;

                bot.edit_message_reply_markup(msg.chat.id, msg.id)
                    .reply_markup(utils::make_subjects_keyboard(
                        subjects,
                        utils::SubjectsKeyboardType::User,
                    ))
                    .await?;

                profile.subjects = Some(subjects.bits());
                dialogue.update(State::SetSubjects(profile)).await?;
            }
        }
        // Subjects filter
        'd' => {
            let mut profile = get_profile(&state)?;

            let subjects_filter = match profile.subjects_filter {
                Some(s) => Subjects::from_bits(s)
                    .context("subjects must be created")?,
                None => Subjects::empty(),
            };

            if last_chars == "continue" {
                bot.edit_message_reply_markup(msg.chat.id, msg.id).await?;

                let subjects_filter_str = if subjects_filter.is_empty() {
                    "Не важно, что ботает другой человек.".to_owned()
                } else {
                    format!(
                        "Предметы, хотя бы один из которых должен ботать тот, \
                         кого вы ищете: {subjects_filter}.",
                    )
                };
                bot.edit_message_text(
                    msg.chat.id,
                    msg.id,
                    format!("{subjects_filter_str}",),
                )
                .await?;

                profile.subjects_filter = Some(subjects_filter.bits());
                next_state(&dialogue, &msg.chat, &state, profile, &bot, &db)
                    .await?;
            } else {
                let subjects_filter = subjects_filter
                    ^ Subjects::from_bits(last_chars.parse()?)
                        .context("subjects error")?;

                bot.edit_message_reply_markup(msg.chat.id, msg.id)
                    .reply_markup(utils::make_subjects_keyboard(
                        subjects_filter,
                        utils::SubjectsKeyboardType::Partner,
                    ))
                    .await?;

                profile.subjects_filter = Some(subjects_filter.bits());
                dialogue.update(State::SetSubjectsFilter(profile)).await?;
            }
        }
        // Dating response callbacks
        '👎' | '💌' | '👍' | '💔' | '❤' => {
            let id = last_chars.parse()?;
            let dating = db.get_dating(id).await?;

            match first_char {
                '👎' => {
                    bot.edit_message_reply_markup(msg.chat.id, msg.id).await?;

                    if dating.initiator_reaction.is_some() {
                        bail!("user abuses dislikes")
                    }

                    db.set_dating_initiator_reaction(id, false).await?;
                    crate::datings::send_recommendation(
                        &bot,
                        &db,
                        ChatId(dating.initiator_id),
                    )
                    .await?;
                }
                '💌' => {
                    bot.edit_message_reply_markup(msg.chat.id, msg.id).await?;

                    if dating.initiator_reaction.is_some() {
                        bail!("user abuses msglikes")
                    }

                    let state = State::LikeMessage { dating };
                    crate::handle::print_current_state(
                        &state, None, &bot, &msg.chat,
                    )
                    .await?;
                    dialogue.update(state).await?;
                }
                '👍' => {
                    bot.edit_message_reply_markup(msg.chat.id, msg.id).await?;

                    if dating.initiator_reaction.is_some() {
                        bail!("user abuses likes")
                    }

                    db.set_dating_initiator_reaction(id, true).await?;
                    crate::datings::send_recommendation(
                        &bot,
                        &db,
                        ChatId(dating.initiator_id),
                    )
                    .await?;
                    crate::datings::send_like(&db, &bot, &dating, None).await?;
                }
                '💔' => {
                    if dating.partner_reaction.is_some() {
                        bail!("partner abuses dislikes")
                    }

                    bot.edit_message_reply_markup(msg.chat.id, msg.id).await?;
                    db.set_dating_partner_reaction(id, false).await?
                }
                '❤' => {
                    if dating.partner_reaction.is_some() {
                        bail!("partner abuses likes")
                    }

                    let initiator = db
                        .get_user(dating.initiator_id)
                        .await?
                        .context("dating initiator not found")?;

                    let partner_keyboard =
                        vec![vec![InlineKeyboardButton::url(
                            "Открыть чат",
                            crate::utils::user_url(&bot, initiator.id)
                                .await?
                                .context("can't get url")?,
                        )]];
                    let partner_keyboard_markup =
                        InlineKeyboardMarkup::new(partner_keyboard);
                    if let Err(e) = bot
                        .edit_message_reply_markup(msg.chat.id, msg.id)
                        .reply_markup(partner_keyboard_markup)
                        .await
                    {
                        sentry_anyhow::capture_anyhow(
                            &anyhow::Error::from(e).context(
                                "error editing mutual like partner's message",
                            ),
                        );
                    }

                    crate::datings::mutual_like(&bot, &db, &dating).await?;
                }
                _ => bail!("unknown callback"),
            }
        }
        _ => bail!("unknown callback"),
    }

    Ok(())
}
