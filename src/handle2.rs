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
    cities::{self, City},
    db,
    handle::next_state,
    request, text,
    types::{Grade, GraduationYear},
    utils, Bot, EditProfile, MyDialogue, State,
};

#[derive(thiserror::Error, Debug)]
enum HandleError {
    #[error("Отправьте текст")]
    NeedText,
    #[error("Отправьте текст")]
    WrongText,
    #[error("Неправильная длина сообщения")]
    Length,
    #[error("Попробуйте ещё раз")]
    Retry,
    #[error("Ignore an error")]
    Ignore,
}

// impl State {
//     fn get_msg(&self) -> anyhow::Result<String> {
//         use State::*;
//         Ok(match self {
//             SetName(p) => format!("SetName: {p:?}"),
//             _ => bail!(""),
//         })
//     }
// }

async fn handle_error(
    e: anyhow::Error,
    bot: &Bot,
    state: &State,
    chat: &Chat,
) -> anyhow::Result<()> {
    use HandleError::*;
    match e.downcast_ref::<HandleError>() {
        Some(e) => match e {
            NeedText | WrongText | Length | Retry => {
                print_state(state, bot, chat).await?;
            }
            Ignore => {}
        },
        None => return Err(e),
    }
    Ok(())
}

#[instrument(level = "debug", skip(db, bot))]
async fn handle_message(
    db: Database,
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
async fn handle_callback(
    db: Database,
    bot: Bot,
    dialogue: MyDialogue,
    mut state: State,
    q: CallbackQuery,
) -> anyhow::Result<()> {
    let chat = q.message.context("callback message is None")?.chat;
    let data = q.data.context("callback data is None")?;
    if let Err(e) =
        try_handle_callback(&db, &bot, &mut state, &chat, &data).await
    {
        handle_error(e, &bot, &state, &chat).await?;
    }
    dialogue.update(state).await?;
    Ok(())
}

async fn print_state(
    state: &State,
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
        SetSubjects(p) => request_set_subjects(bot, chat, p).await?,
        SetSubjectsFilter(p) => {
            request_set_subjects_filter(bot, chat, p).await?
        }
        SetDatingPurpose(p) => request_set_dating_purpose(bot, chat, p).await?,
        SetCity(_) => request_set_city(bot, chat).await?,
        SetLocationFilter(p) => {
            request_set_location_filter(bot, chat, p).await?
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

async fn try_handle_message(
    db: &Database,
    bot: &Bot,
    state: &mut State,
    msg: &Message,
) -> anyhow::Result<()> {
    let chat = &msg.chat;
    let t = msg.text();

    macro_rules! send {
        ($e:expr) => {
            bot.send_message(chat.id, $e).await?;
        };
        ($e:expr, remove) => {
            bot.send_message(chat.id, $e)
                .reply_markup(KeyboardRemove::new())
                .await?;
        };
        ($e:expr, markup $k:expr) => {
            bot.send_message(chat.id, $e)
                .reply_markup(KeyboardMarkup::new($k).resize_keyboard(true))
                .await?;
        };
        ($e:expr, inline $k:expr) => {
            bot.send_message(chat.id, $e)
                .reply_markup(InlineKeyboardMarkup::new($k))
                .await?;
        };
    }

    macro_rules! upd_print {
        ($e:expr) => {
            let e = $e;
            print_state(&e, bot, chat).await?;
            *state = e;
        };
    }

    // send!(id, "text", markup [[KeyboardButton::new("button")]]);
    // enum Next {
    //     Retry,
    //     Switch,
    // }

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
            // TODO: skip button?
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

        // explicit ignore (for now)
        SetSubjects(_)
        | SetSubjectsFilter(_)
        | SetDatingPurpose(_)
        | LikeMessage { .. }
        | Edit => {}
    }
    Ok(())
}

async fn try_handle_callback(
    db: &Database,
    bot: &Bot,
    state: &mut State,
    chat: &Chat,
    data: &str,
) -> anyhow::Result<()> {
    // Why macro? Because async closures are unstable,
    // the only difference is "!"
    macro_rules! upd_print {
        ($e:expr) => {
            let e = $e;
            print_state(&e, bot, chat).await?;
            *state = e;
        };
    }

    // TODO: everything

    use State::*;
    match state {
        SetSubjects(p) => {
            // upd_print!(if p.create_new {
            //     SetSubjectsFilter(mem::take(p))
            // } else {
            //     Start
            // });
            upd_print!(SetSubjectsFilter(mem::take(p)));
        }
        SetSubjectsFilter(p) => {
            upd_print!(if p.create_new {
                SetDatingPurpose(mem::take(p))
            } else {
                Start
            });
        }
        SetDatingPurpose(p) => {
            upd_print!(if p.create_new {
                SetCity(mem::take(p))
            } else {
                Start
            });
        }
        LikeMessage { dating } => {}
        Edit => {}

        // explicit ignore
        Start | SetName(_) | SetGender(_) | SetGenderFilter(_)
        | SetGraduationYear(_) | SetCity(_) | SetLocationFilter(_)
        | SetAbout(_) | SetPhotos(_) => {}
    }

    Ok(())
}
