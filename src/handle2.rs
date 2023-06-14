use std::sync::Arc;

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
    // #[error("Wrong state: {0:?}")]
    // WrongState(State),
    // #[error(transparent)]
    // Other(#[from] anyhow::Error),
}

impl State {
    fn get_msg(&self) -> anyhow::Result<String> {
        use State::*;
        Ok(match self {
            SetName(p) => format!("SetName: {p:?}"),
            _ => bail!(""),
        })
    }
}

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
    state: State,
    chat: &Chat,
    text: Option<&str>,
) -> anyhow::Result<()> {
    // ensure!(chat.is_private(), "chat isn't private");
    if let Err(e) = try_handle(db, bot, dialogue, state.clone(), chat, text).await {
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
    dialogue: &MyDialogue,
    state: State,
    chat: &Chat,
    t: Option<&str>,
) -> anyhow::Result<()> {
    // let id = chat.id;

    // Why macros? Because async closures are unstable,
    // the only difference is "!"
    // macro_rules! upd {
    //     ($e:expr) => {
    //         dialogue.update($e).await?;
    //     };
    // }
    macro_rules! upd_print {
        ($e:expr) => {
            let e = $e;
            print_state(&e, bot, chat).await?;
            dialogue.update(e).await?;
        };
    }

    // send!(id, "text", markup [[KeyboardButton::new("button")]]);
    // enum Next {
    //     Retry,
    //     Switch,
    // }

    use State::*;
    match state {
        SetName(mut p) => {
            let t = t.ok_or(HandleError::NeedText)?;
            ensure!((3..=16).contains(&t.chars().count()), HandleError::Length);
            p.name = Some(t.to_owned());
            upd_print!(if p.create_new { SetGender(p) } else { Start });
        }
        SetGender(mut p) => {
            let t = t.ok_or(HandleError::NeedText)?;
            let gender = match t {
                text::GENDER_MALE => Gender::Male,
                text::GENDER_FEMALE => Gender::Female,
                _ => bail!(HandleError::WrongText),
            };
            p.gender = Some(gender);
            upd_print!(if p.create_new { SetGenderFilter(p) } else { Start });
        }
        SetGenderFilter(mut p) => {
            let t = t.ok_or(HandleError::NeedText)?;
            let gender = match t {
                text::GENDER_FILTER_MALE => Some(Gender::Male),
                text::GENDER_FILTER_FEMALE => Some(Gender::Female),
                text::GENDER_FILTER_ANY => None,
                _ => bail!(HandleError::WrongText),
            };
            p.gender_filter = Some(gender);
            upd_print!(if p.create_new { SetGraduationYear(p) } else { Start });
        }
        SetGraduationYear(mut p) => {
            let t = t.ok_or(HandleError::NeedText)?;
            let grade = t.parse::<i8>().map_err(|_| HandleError::WrongText)?;
            let grade =
                Grade::try_from(grade).map_err(|_| HandleError::WrongText)?;
            let graduation_year: GraduationYear = grade.into();
            p.graduation_year = Some(graduation_year.into());
            upd_print!(if p.create_new { SetSubjects(p) } else { Start });
        }
        // TODO
        s => bail!("wrong state: {s:?}"),
    };
    Ok(())
}
