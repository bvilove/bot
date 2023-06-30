use std::{mem, sync::Arc};

use anyhow::{bail, ensure, Context};
use db::Database;
use entities::sea_orm_active_enums::{Gender, ImageKind, LocationFilter};
use teloxide::{
    // net::Download,
    prelude::*,
    types::{
        Chat, InlineKeyboardButton, InlineKeyboardMarkup, KeyboardButton,
        KeyboardMarkup, KeyboardRemove,
    },
    utils::command::BotCommands,
};
use tracing::instrument;

use crate::{
    callbacks::{Callback, RateCode, UpdateBitflags},
    cities::{self, City},
    db, text,
    types::{DatingPurpose, Grade, GraduationYear, Subjects},
    utils, Bot, EditProfile, MyDialogue, State,
};

#[derive(thiserror::Error, Debug)]
enum HandleError {
    #[error("отправьте текст")]
    NeedText,
    #[error("неправильный текст")]
    WrongText,
    #[error("неправильная длина сообщения")]
    Length,
    #[error("попробуйте ещё раз")]
    Retry,
    #[error("ignore an error")]
    Ignore,
    #[error("wrong callback code")]
    WrongCode,
}

macro_rules! make_macros {
    ($bot:ident, $msg:ident, $state:ident, $chat:ident) => {
        // Why macros? Because async closures are unstable,
        // the only difference is "!"
        macro_rules! upd_print {
            ($e:expr) => {
                let e = $e;
                crate::handle::print_state(&e, $bot, $chat).await?;
                *$state = e;
            };
        }
        // NOTE: not removing buttons is considered a bug!
        macro_rules! remove_buttons {
            () => {
                $bot.edit_message_reply_markup($chat.id, $msg.id).await?;
            };
        }
        macro_rules! send {
            ($e:expr) => {
                $bot.send_message($chat.id, $e).await?;
            };
            ($e:expr, remove) => {
                $bot.send_message($chat.id, $e)
                    .reply_markup(KeyboardRemove::new())
                    .await?;
            };
            ($e:expr, markup $k:expr) => {
                $bot.send_message($chat.id, $e)
                    .reply_markup(KeyboardMarkup::new($k).resize_keyboard(true))
                    .await?;
            };
            ($e:expr, inline $k:expr) => {
                $bot.send_message($chat.id, $e)
                    .reply_markup(InlineKeyboardMarkup::new($k))
                    .await?;
            };
        }
    };
}
pub(crate) use make_macros;

async fn handle_error(
    e: anyhow::Error,
    bot: &Bot,
    state: &State,
    chat: &Chat,
) -> anyhow::Result<()> {
    use HandleError::*;
    match e.downcast_ref::<HandleError>() {
        Some(h) => match h {
            NeedText | WrongText | Length | Retry => {
                print_state(state, bot, chat).await?;
            }
            Ignore => {}
            WrongCode => return Err(e),
        },
        None => return Err(e),
    }
    Ok(())
}

#[instrument(level = "debug", skip(db, bot))]
pub async fn handle_message(
    db: Arc<Database>,
    bot: Bot,
    dialogue: MyDialogue,
    mut state: State,
    msg: Message,
) -> anyhow::Result<()> {
    if let Err(e) = try_handle_message(&db, &bot, &mut state, &msg).await {
        handle_error(e, &bot, &state, &msg.chat).await?;
    }
    dialogue.update(state).await?;
    Ok(())
}

#[instrument(level = "debug", skip(db, bot))]
pub async fn handle_callback(
    db: Arc<Database>,
    bot: Bot,
    dialogue: MyDialogue,
    mut state: State,
    q: CallbackQuery,
) -> anyhow::Result<()> {
    let msg = q.message.as_ref().context("callback message is None")?;
    let data = q.data.as_deref().context("callback data is None")?;
    if let Err(e) =
        try_handle_callback(&db, &bot, &mut state, msg, data, &q).await
    {
        handle_error(e, &bot, &state, &msg.chat).await?;
    }
    dialogue.update(state).await?;
    Ok(())
}

/// Send a message to the user about the current dialog state
pub async fn print_state(
    state: &State,
    bot: &Bot,
    chat: &Chat,
) -> anyhow::Result<()> {
    use State::*;

    use crate::request::*;

    match state {
        // edit profile
        SetName(_) => set_name(bot, chat).await?,
        SetGender(_) => set_gender(bot, chat).await?,
        SetGenderFilter(_) => set_gender_filter(bot, chat).await?,
        SetGraduationYear(_) => set_grade(bot, chat).await?,
        SetSubjects(p) => set_subjects(bot, chat, p).await?,
        SetSubjectsFilter(p) => {
            set_subjects_filter(bot, chat, p).await?;
        }
        SetDatingPurpose(p) => set_dating_purpose(bot, chat, p).await?,
        SetCity(_) => set_city(bot, chat).await?,
        SetLocationFilter(p) => {
            set_location_filter(bot, chat, p).await?;
        }
        SetAbout(_) => set_about(bot, chat).await?,
        SetPhotos(_) => set_photos(bot, chat).await?,
        // others
        LikeWithMessage { .. } => {
            crate::datings::request_like_msg(bot, chat).await?;
        }
        Edit => edit_profile(bot, chat).await?,
        // invalid states
        Start => {}
    };
    Ok(())
}

async fn try_handle_message(
    db: &Database,
    bot: &Bot,
    state: &mut State,
    msg: &Message,
) -> anyhow::Result<()> {
    let chat = &msg.chat;
    let t = msg.text();

    make_macros!(bot, msg, state, chat);

    use State::*;
    match state {
        SetName(p) => {
            let t = t.ok_or(HandleError::NeedText)?;
            ensure!((3..=16).contains(&t.chars().count()), HandleError::Length);
            p.name = Some(t.to_owned());
            upd_print!(if p.create_new {
                SetGender(mem::take(p))
            } else {
                Start
            });
        }
        SetGender(p) => {
            let t = t.ok_or(HandleError::NeedText)?;
            let gender = match t {
                text::GENDER_MALE => Gender::Male,
                text::GENDER_FEMALE => Gender::Female,
                _ => bail!(HandleError::WrongText),
            };
            p.gender = Some(gender);
            upd_print!(if p.create_new {
                SetGenderFilter(mem::take(p))
            } else {
                Start
            });
        }
        SetGenderFilter(p) => {
            let t = t.ok_or(HandleError::NeedText)?;
            let gender = match t {
                text::GENDER_FILTER_MALE => Some(Gender::Male),
                text::GENDER_FILTER_FEMALE => Some(Gender::Female),
                text::GENDER_FILTER_ANY => None,
                _ => bail!(HandleError::WrongText),
            };
            p.gender_filter = Some(gender);
            upd_print!(if p.create_new {
                SetGraduationYear(mem::take(p))
            } else {
                Start
            });
        }
        SetGraduationYear(p) => {
            let t = t.ok_or(HandleError::NeedText)?;
            let grade = t.parse::<i8>().map_err(|_| HandleError::WrongText)?;
            let grade =
                Grade::try_from(grade).map_err(|_| HandleError::WrongText)?;
            let graduation_year: GraduationYear = grade.into();
            p.graduation_year = Some(graduation_year.into());
            upd_print!(if p.create_new {
                SetSubjects(mem::take(p))
            } else {
                Start
            });
        }
        SetCity(p) => {
            let t = t.ok_or(HandleError::NeedText)?;

            match t {
                "Верно" if p.city.is_some() => {
                    upd_print!(SetLocationFilter(mem::take(p)));
                }
                "Не указывать" => {
                    p.city = Some(None);
                    p.location_filter = Some(LocationFilter::SameCountry);

                    send!(text::NO_CITY, remove);
                    upd_print!(if p.create_new {
                        SetAbout(mem::take(p))
                    } else {
                        Start
                    });
                }
                city => {
                    if let Ok(city) = city.parse::<City>() {
                        p.city = Some(city.into());
                        send!(
                            format!("Ваш город - {city}?"),
                            markup[[
                                KeyboardButton::new("Верно"),
                                KeyboardButton::new("Не указывать"),
                            ]]
                        );
                    } else {
                        send!(
                            text::CANT_FIND_CITY,
                            markup[[KeyboardButton::new("Не указывать")]]
                        );
                    }
                }
            }
        }
        SetLocationFilter(p) => {
            let t = t.ok_or(HandleError::NeedText)?;
            // let filter = match t.chars().next() {
            //     Some('1') => LocationFilter::SameCountry,
            //     Some('2') => LocationFilter::SameCounty,
            //     Some('3') => LocationFilter::SameSubject,
            //     Some('4') => LocationFilter::SameCity,
            //     _ => bail!(HandleError::WrongText),
            // };

            // TODO: fix this mostrosity
            let filter = if t == "Вся Россия" {
                LocationFilter::SameCountry
            } else if cities::county_exists(
                &t.chars()
                    .rev()
                    .skip(3)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect::<String>(),
            ) {
                LocationFilter::SameCounty
            } else if cities::subject_exists(t) {
                LocationFilter::SameSubject
            } else if cities::city_exists(t) {
                LocationFilter::SameCity
            } else {
                bail!(HandleError::WrongText);
            };

            p.location_filter = Some(filter);
            upd_print!(if p.create_new {
                SetAbout(mem::take(p))
            } else {
                Start
            });
        }
        SetAbout(p) => {
            let t = t.ok_or(HandleError::NeedText)?;
            ensure!(
                (1..=1024).contains(&t.chars().count()),
                HandleError::Length
            );
            p.about = Some(t.to_owned());
            // FIXME: HACK: create user before SetPhotos
            db.create_or_update_user(p.clone()).await?;
            upd_print!(if p.create_new {
                SetPhotos(mem::take(p))
            } else {
                Start
            });
        }
        SetPhotos(p) => match t {
            Some("Без фото") => {
                db.clean_images(chat.id.0).await?;
                crate::datings::send_profile(bot, db, p.id).await?;
                upd_print!(Start);
            }
            Some("Сохранить") => {
                crate::datings::send_profile(bot, db, p.id).await?;
                upd_print!(Start);
            }
            _ => {
                // TODO: change type of photos_count to Option<u8>
                // TODO: reset photos button
                if p.photos_count == 0 {
                    db.clean_images(msg.chat.id.0).await?;
                } else if p.photos_count >= 10 {
                    send!(
                        "Невозможно добавить более 10 фото/видео",
                        markup[[KeyboardButton::new("Сохранить")]]
                    );
                    return Ok(());
                };

                if let Some([.., photo]) = msg.photo() {
                    let file = bot.get_file(&photo.file.id).await?;
                    db.create_image(chat.id.0, file.meta.id, ImageKind::Image)
                        .await?;
                } else if let Some(video) = msg.video() {
                    let file = bot.get_file(&video.file.id).await?;
                    db.create_image(chat.id.0, file.meta.id, ImageKind::Video)
                        .await?;
                } else {
                    bail!(HandleError::WrongText);
                };

                p.photos_count += 1;

                send!(
                    format!(
                        "Добавлено {}/10 фото/видео. Добавить ещё?",
                        p.photos_count
                    ),
                    markup[[KeyboardButton::new("Сохранить")]]
                );
            }
        },
        // TODO: confirm profile change State
        Start => {
            bot.send_message(
                chat.id,
                crate::Command::descriptions().to_string(),
            )
            .await?;
        }
        LikeWithMessage { dating } => {
            let t = t.ok_or(HandleError::NeedText)?;

            let msg_to_send = if t == "Отмена"
                || t.chars().next().context("empty string")? == '/'
            {
                db.set_dating_initiator_reaction(dating.id, false).await?;
                "Отправка лайка отменена"
            } else {
                db.set_dating_initiator_reaction(dating.id, true).await?;
                crate::datings::send_like(db, bot, dating, Some(t.to_owned()))
                    .await?;
                "Лайк отправлен!"
            };

            send!(msg_to_send, remove);
            crate::datings::send_recommendation(
                bot,
                db,
                ChatId(dating.initiator_id),
            )
            .await?;
            upd_print!(Start);
        }

        // explicit ignore (for now)
        SetSubjects(_) | SetSubjectsFilter(_) | SetDatingPurpose(_) | Edit => {}
    }
    Ok(())
}

async fn try_handle_callback(
    db: &Database,
    bot: &Bot,
    state: &mut State,
    msg: &Message,
    data: &str,
    q: &CallbackQuery,
) -> anyhow::Result<()> {
    let chat = &msg.chat;
    make_macros!(bot, msg, state, chat);

    let callback: Callback = data.parse()?;

    use State::*;

    if matches!(callback, Callback::Dating { .. }) && *state != Start {
        bot.answer_callback_query(&q.id)
            .text("Сначала выйдите из режима редактирования!")
            .show_alert(true)
            .await?;
        return Ok(());
    }

    match state {
        SetSubjects(p) => {
            let Callback::SetSubjects(changed_subjects) = callback else {
                bail!("wrong callback type")
            };

            // FIXME: store Subjects in EditProfile
            let current_subjects = match p.subjects {
                Some(s) => Subjects::from_bits(s)
                    .context("subjects must be created")?,
                None => Subjects::empty(),
            };

            match changed_subjects {
                UpdateBitflags::Continue => {
                    remove_buttons!();

                    let subjects_str = if current_subjects.is_empty() {
                        "Вы ничего не ботаете.".to_owned()
                    } else {
                        format!(
                            "Предметы, которые вы ботаете: {current_subjects}.",
                        )
                    };
                    bot.edit_message_text(msg.chat.id, msg.id, subjects_str)
                        .await?;

                    p.subjects = Some(current_subjects.bits());
                    upd_print!(SetSubjectsFilter(mem::take(p)));
                }
                UpdateBitflags::Update(changed_subjects) => {
                    let new_subjects = current_subjects ^ changed_subjects;

                    bot.edit_message_reply_markup(msg.chat.id, msg.id)
                        .reply_markup(utils::make_subjects_keyboard(
                            new_subjects,
                            &utils::SubjectsKeyboardType::User,
                        ))
                        .await?;

                    p.subjects = Some(new_subjects.bits());
                }
            }
        }
        SetSubjectsFilter(p) => {
            let Callback::SetSubjectsFilter(changed_subjects_filter) = callback else {
                bail!("wrong callback type")
            };

            // FIXME: store Subjects in EditProfile
            let current_filter = match p.subjects_filter {
                Some(s) => Subjects::from_bits(s)
                    .context("subjects must be created")?,
                None => Subjects::empty(),
            };

            match changed_subjects_filter {
                UpdateBitflags::Continue => {
                    remove_buttons!();

                    let subjects_filter_str = if current_filter.is_empty() {
                        "Не важно, что ботает другой человек.".to_owned()
                    } else {
                        format!(
                            "Предметы, хотя бы один из которых должен ботать \
                             тот, кого вы ищете: {current_filter}.",
                        )
                    };
                    bot.edit_message_text(
                        msg.chat.id,
                        msg.id,
                        subjects_filter_str,
                    )
                    .await?;

                    p.subjects_filter = Some(current_filter.bits());
                    upd_print!(SetSubjectsFilter(mem::take(p)));
                }
                UpdateBitflags::Update(changed_subjects_filter) => {
                    let new_subjects_filter =
                        current_filter ^ changed_subjects_filter;

                    bot.edit_message_reply_markup(msg.chat.id, msg.id)
                        .reply_markup(utils::make_subjects_keyboard(
                            changed_subjects_filter,
                            &utils::SubjectsKeyboardType::User,
                        ))
                        .await?;

                    p.subjects_filter = Some(new_subjects_filter.bits());
                }
            }
        }
        SetDatingPurpose(p) => {
            let Callback::SetDatingPurpose(new_purpose) = callback else {
                bail!("wrong callback type")
            };

            // FIXME: store DatingPurpose in EditProfile
            let current_purpose = match p.dating_purpose {
                Some(s) => DatingPurpose::try_from(s)?,
                None => DatingPurpose::empty(),
            };

            match new_purpose {
                UpdateBitflags::Continue => {
                    ensure!(
                        !current_purpose.is_empty(),
                        "there must be at least 1 purpose"
                    );
                    remove_buttons!();

                    bot.edit_message_text(
                        msg.chat.id,
                        msg.id,
                        format!("Вас интересует: {current_purpose}.",),
                    )
                    .await?;

                    p.dating_purpose = Some(current_purpose.bits());
                    upd_print!(if p.create_new {
                        SetCity(mem::take(p))
                    } else {
                        Start
                    });
                }
                UpdateBitflags::Update(changed_purpose) => {
                    let new_purpose = current_purpose ^ changed_purpose;

                    bot.edit_message_reply_markup(msg.chat.id, msg.id)
                        .reply_markup(utils::make_dating_purpose_keyboard(
                            new_purpose,
                        ))
                        .await?;

                    p.dating_purpose = Some(new_purpose.bits());
                }
            }
        }
        Edit => {
            // TODO: edit should work in Start state
            // ensure!(code == Callback::Edit, HandleError::WrongCode);
            // TODO: strum on State?
            // FIXME: check if user exists
            let user =
                db.get_user(msg.chat.id.0).await?.context("user not found")?;
            let p = EditProfile::from_model(user); // FIXME: why?

            remove_buttons!();
            let state = match data {
                "Имя" => SetName(p),
                "Предметы" => SetSubjects(p),
                "О себе" => SetAbout(p),
                "Город" => SetCity(p),
                "Фото" => SetPhotos(p),
                "Отмена" => Start,
                _ => bail!("unknown edit data"),
            };
            upd_print!(state);
        }
        Start => {
            match callback {
                Callback::Edit => todo!(),
                Callback::Dating { dating_id, code } => {
                    let dating = db.get_dating(dating_id).await?;
                    match code {
                        RateCode::Dislike => {
                            remove_buttons!();
                            ensure!(
                                dating.initiator_reaction.is_none(),
                                "user abuses dislikes"
                            );
                            db.set_dating_initiator_reaction(dating_id, false)
                                .await?;
                            crate::datings::send_recommendation(
                                bot,
                                db,
                                ChatId(dating.initiator_id),
                            )
                            .await?;
                        }
                        RateCode::LikeWithMsg => {
                            remove_buttons!();
                            ensure!(
                                dating.initiator_reaction.is_none(),
                                "user abuses msglikes"
                            );
                            upd_print!(State::LikeWithMessage { dating });
                        }
                        RateCode::Like => {
                            remove_buttons!();
                            ensure!(
                                dating.initiator_reaction.is_none(),
                                "user abuses likes"
                            );

                            db.set_dating_initiator_reaction(dating_id, true)
                                .await?;
                            crate::datings::send_recommendation(
                                bot,
                                db,
                                ChatId(dating.initiator_id),
                            )
                            .await?;
                            crate::datings::send_like(db, bot, &dating, None)
                                .await?;
                        }
                        RateCode::ResponseDislike => {
                            remove_buttons!();
                            ensure!(
                                dating.partner_reaction.is_none(),
                                "partner abuses dislikes"
                            );
                            db.set_dating_partner_reaction(dating_id, false)
                                .await?;
                        }
                        RateCode::ResponseLike => {
                            ensure!(
                                dating.partner_reaction.is_none(),
                                "partner abuses likes"
                            );

                            let initiator = db
                                .get_user(dating.initiator_id)
                                .await?
                                .context("dating initiator not found")?;

                            let markup = InlineKeyboardMarkup::new([[
                                InlineKeyboardButton::url(
                                    "Открыть чат",
                                    crate::utils::user_url(bot, initiator.id)
                                        .await?
                                        .context("can't get url")?,
                                ),
                            ]]);

                            crate::datings::mutual_like(bot, db, &dating)
                                .await?;
                            // TODO: check if error works
                            bot.edit_message_reply_markup(msg.chat.id, msg.id)
                                .reply_markup(markup)
                                .await
                                .context(
                                    "error editing mutual like partner's \
                                     message",
                                )?;
                        }
                    }
                }
                Callback::CreateProfile => {
                    crate::start_profile_creation(state, msg, bot).await?;
                }
                Callback::FindPartner => {
                    remove_buttons!();
                    // TODO: refactor this
                    crate::datings::send_recommendation(bot, db, msg.chat.id)
                        .await?;
                }
                _ => bail!("wrong callback type"),
            }
        }
        // explicit ignore
        SetName(_)
        | SetGender(_)
        | SetGenderFilter(_)
        | SetGraduationYear(_)
        | SetCity(_)
        | SetLocationFilter(_)
        | SetAbout(_)
        | SetPhotos(_)
        | LikeWithMessage { .. } => {}
    }

    Ok(())
}
