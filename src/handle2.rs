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
    }, utils::command::BotCommands,
};
use tracing::instrument;

use crate::{
    cities, db,
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

macro_rules! send {
    ($id:ident, $e:expr) => {
        bot.send_message($id, $e).await?;
    };
    ($id:ident, $e:expr, remove) => {
        bot.send_message($id, $e)
            .reply_markup(KeyboardRemove::new())
            .await?;
    };
    ($id:ident, $e:expr, markup $k:expr) => {
        bot.send_message($id, $e)
            .reply_markup(KeyboardMarkup::new($k).resize_keyboard(true))
            .await?;
    };
    ($id:ident, $e:expr, inline $k:expr) => {
        bot.send_message($id, $e)
            .reply_markup(InlineKeyboardMarkup::new($k))
            .await?;
    };
}

async fn handle_message(
    db: Database,
    bot: Bot,
    dialogue: MyDialogue,
    state: State,
    msg: Message,
) -> anyhow::Result<()> {
    handle(&db, &bot, &dialogue, state, &msg.chat, msg.text()).await
}

async fn handle_callback(
    db: Database,
    bot: Bot,
    dialogue: MyDialogue,
    state: State,
    q: CallbackQuery,
) -> anyhow::Result<()> {
    handle(
        &db,
        &bot,
        &dialogue,
        state,
        &q.message.as_ref().context("message is None")?.chat,
        q.data.as_deref(),
    )
    .await
}

#[instrument(level = "debug", skip(db, bot))]
async fn handle(
    db: &Database,
    bot: &Bot,
    dialogue: &MyDialogue,
    mut state: State,
    chat: &Chat,
    text: Option<&str>,
) -> anyhow::Result<()> {
    // TODO: ensure!(chat.is_private(), "chat isn't private");
    if let Err(e) = try_handle(db, bot, &mut state, chat, text).await
    {
        use HandleError::*;
        match e.downcast_ref::<HandleError>() {
            Some(e) => match e {
                NeedText | WrongText | Length | Retry => {
                    print_state(&state, bot, chat).await?;
                }
                Ignore => {}
            },
            None => return Err(e),
        }
    }
    // TODO: always updates the dialogue, is is bad?
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

async fn try_handle(
    db: &Database,
    bot: &Bot,
    state: &mut State,
    chat: &Chat,
    t: Option<&str>,
) -> anyhow::Result<()> {
    // let id = chat.id;

    // Why macros? Because async closures are unstable,
    // the only difference is "!"
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
        // TODO: callback SetSubjects(p) => {
        //     // upd_print!(if p.create_new {
        //     //     SetSubjectsFilter(mem::take(p))
        //     // } else {
        //     //     Start
        //     // });
        //     upd_print!(SetSubjectsFilter(mem::take(p)));
        // }
        // TODO: callback SetSubjectsFilter(p) => {
        //     upd_print!(if p.create_new {
        //         SetDatingPurpose(mem::take(p))
        //     } else {
        //         Start
        //     });
        // }
        // TODO: callback SetDatingPurpose(p) => {
        //     upd_print!(if p.create_new {
        //         SetCity(mem::take(p))
        //     } else {
        //         Start
        //     });
        // }
        SetCity(p) => {
            upd_print!(if p
                .city
                .context("city must be set when city editing finished")?
                .is_some()
            {
                SetLocationFilter(mem::take(p))
            } else if p.create_new {
                SetAbout(mem::take(p))
            } else {
                Start
            });
        }
        SetLocationFilter(p) => {
            let t = t.ok_or(HandleError::NeedText)?;
            // FIXME: fix request_location_filter
            let filter = match t.chars().next() {
                Some('1') => LocationFilter::SameCountry,
                Some('2') => LocationFilter::SameCounty,
                Some('3') => LocationFilter::SameSubject,
                Some('4') => LocationFilter::SameCity,
                _ => bail!(HandleError::WrongText),
            };
            
            // if text == "Вся Россия" {
            //     LocationFilter::SameCountry
            // } else if cities::county_exists(
            //     &text
            //         .chars()
            //         .rev()
            //         .skip(3)
            //         .collect::<Vec<_>>()
            //         .into_iter()
            //         .rev()
            //         .collect::<String>(),
            // ) {
            //     LocationFilter::SameCounty
            // } else if cities::subject_exists(text) {
            //     LocationFilter::SameSubject
            // } else if cities::city_exists(text) {
            //     LocationFilter::SameCity
            // } else {
            //     print_current_state(&state, Some(&profile), &bot, &msg.chat).await?;
            //     return Ok(());
            // };
        
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
            // FIXME: 1024 top limit?
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
        // TODO: message SetPhotos(p) => {
        //     crate::datings::send_profile(bot, db, p.id).await?;
        //     *state = Start;
        // }
        // TODO: callback LikeMessage { dating } => {}
        // TODO: callback Edit => {}
        Start => {
            // FIXME: fix callback case
            bot.send_message(chat.id, crate::Command::descriptions().to_string()).await?;
        }
        s => bail!("unimplemented: {s:?}"),
    }
    Ok(())
}
