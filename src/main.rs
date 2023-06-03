use std::fmt::Display;

use anyhow::Context;
use bitflags::bitflags;
use entities::sea_orm_active_enums::Gender;
use itertools::Itertools;
use teloxide::{
    adaptors::{throttle::Limits, Throttle},
    dispatching::dialogue::InMemStorage,
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup},
    utils::command::BotCommands,
};

type Bot = Throttle<teloxide::Bot>;
type MyDialogue = Dialogue<State, InMemStorage<State>>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    tracing::info!("Starting bot...");
    let bot = teloxide::Bot::from_env()
        .throttle(Limits { messages_per_min_chat: 30, ..Default::default() });

    let handler = dptree::entry()
        .enter_dialogue::<Update, InMemStorage<State>, State>()
        .branch(
            Update::filter_message()
                .branch(dptree::case![State::NewName(a)].endpoint(new_profile))
                .branch(dptree::case![State::NewAbout(a)].endpoint(new_profile))
                .branch(
                    dptree::entry()
                        .filter_command::<Command>()
                        .endpoint(answer),
                )
                .branch(dptree::endpoint(invalid_command)),
        )
        .branch(
            Update::filter_callback_query()
                .branch(
                    dptree::case![State::NewGender(a)]
                        .endpoint(new_profile_callback),
                )
                .branch(
                    dptree::case![State::NewGrade(a)]
                        .endpoint(new_profile_callback),
                )
                .branch(
                    dptree::case![State::NewSubjects(a)]
                        .endpoint(new_profile_callback),
                )
                .branch(
                    dptree::case![State::NewTargetGender(a)]
                        .endpoint(new_profile_callback),
                ),
        );

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![InMemStorage::<State>::new()])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
    Ok(())
}

bitflags! {
    #[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
    struct Subjects: i64 {
        const Art = 1 << 0;
        const Astronomy = 1 << 1;
        const Biology = 1 << 2;
        const Chemistry = 1 << 3;
        const Chinese = 1 << 4;
        const Ecology = 1 << 5;
        const Economics = 1 << 6;
        const English = 1 << 7;
        const French = 1 << 8;
        const Geography = 1 << 9;
        const German = 1 << 10;
        const History = 1 << 11;
        const Informatics = 1 << 12;
        const Italian = 1 << 13;
        const Law = 1 << 14;
        const Literature = 1 << 15;
        const Math = 1 << 16;
        const Physics = 1 << 17;
        const Russian = 1 << 18;
        const Safety = 1 << 19;
        const Social = 1 << 20;
        const Spanish = 1 << 21;
        const Sport = 1 << 22;
        const Technology = 1 << 23;
    }
}

#[derive(Clone, Default)]
struct NewProfile {
    name: Option<String>,
    gender: Option<Gender>,
    grade: Option<u8>,
    subjects: Option<Subjects>,
    about: Option<String>,
    target_gender: Option<Gender>,
}

#[derive(Debug)]
struct Profile {
    name: String,
    gender: Gender,
    grade: u8,
    subjects: Subjects,
    about: String,
    target_gender: Gender,
}

impl Profile {
    fn try_new(new: NewProfile) -> Option<Self> {
        match new {
            NewProfile {
                name: Some(name),
                gender: Some(gender),
                grade: Some(grade),
                subjects: Some(subjects),
                about: Some(about),
                target_gender: Some(target_gender),
            } => Some(Self {
                name,
                gender,
                grade,
                subjects,
                about,
                target_gender,
            }),
            _ => None,
        }
    }
}

#[derive(Clone, Default)]
enum State {
    #[default]
    Start,
    // NewProfile:
    NewName(NewProfile),
    NewGender(NewProfile),
    NewGrade(NewProfile),
    NewSubjects(NewProfile),
    NewAbout(NewProfile),
    NewTargetGender(NewProfile),
    // EditProfile:
    // TODO
}

async fn new_profile_callback(
    bot: Bot,
    dialogue: MyDialogue,
    mut profile: NewProfile,
    state: State,
    q: CallbackQuery,
) -> anyhow::Result<()> {
    let Some(t) = q.data else { return Ok(()); };
    const GRADES: [&str; 4] = ["8", "9", "10", "11"];
    const SUBJECTS_CONTINUE: &str = "Продолжить";

    fn parse_gender(t: &str) -> anyhow::Result<Gender> {
        if t == Gender::Male.to_string() {
            Ok(Gender::Male)
        } else if t == Gender::Female.to_string() {
            Ok(Gender::Female)
        } else {
            anyhow::bail!("gender error: {t}");
        }
    }

    macro_rules! remove_buttons {
        () => {
            if let Some(Message { id, chat, .. }) = q.message {
                bot.edit_message_reply_markup(chat.id, id).await?;
            }
        };
    }

    fn make_subjects_keyboard(selected: Subjects) -> InlineKeyboardMarkup {
        let mut keyboard: Vec<Vec<InlineKeyboardButton>> = Subjects::all()
            .iter_names()
            .chunks(3)
            .into_iter()
            .map(|row| {
                row.map(|(name, val)| {
                    InlineKeyboardButton::callback(
                        if selected.contains(val) {
                            format!("✅ {}", name)
                        } else {
                            name.to_owned()
                        },
                        val.bits().to_string(),
                    )
                })
                .collect()
            })
            .collect();
        keyboard.push(vec![InlineKeyboardButton::callback(
            SUBJECTS_CONTINUE,
            SUBJECTS_CONTINUE,
        )]);
        InlineKeyboardMarkup::new(keyboard)
    }

    match state {
        State::NewGender(_) => {
            profile.gender = Some(parse_gender(&t)?);
            // bot.answer_callback_query(q.id).await?;

            bot.send_message(dialogue.chat_id(), text::EDIT_GRADE)
                .reply_markup({
                    let keyboard: Vec<Vec<InlineKeyboardButton>> = GRADES
                        .into_iter()
                        .map(|g| vec![InlineKeyboardButton::callback(g, g)])
                        .collect();
                    InlineKeyboardMarkup::new(keyboard)
                })
                .await?;
            remove_buttons!();
            dialogue.update(State::NewGrade(profile)).await?;
        }
        State::NewGrade(_) => {
            if !GRADES.contains(&t.as_str()) {
                anyhow::bail!("grade error: {t}");
            }
            profile.grade = Some(t.parse()?);
            bot.send_message(dialogue.chat_id(), text::EDIT_SUBJECT)
                .reply_markup(make_subjects_keyboard(Subjects::default()))
                .await?;
            remove_buttons!();
            dialogue.update(State::NewSubjects(profile)).await?;
        }
        State::NewSubjects(_) => {
            if t == SUBJECTS_CONTINUE {
                bot.send_message(dialogue.chat_id(), text::EDIT_ABOUT).await?;
                remove_buttons!();
                dialogue.update(State::NewAbout(profile)).await?;
            } else {
                let subjects = profile.subjects.unwrap_or_default()
                    ^ Subjects::from_bits(t.parse()?)
                        .context("subjects error")?;
                dialogue
                    .update(State::NewSubjects(NewProfile {
                        subjects: Some(subjects),
                        ..profile
                    }))
                    .await?;
                if let Some(Message { id, chat, .. }) = q.message {
                    bot.edit_message_reply_markup(chat.id, id)
                        .reply_markup(make_subjects_keyboard(subjects))
                        .await?;
                }
            }
        }
        State::NewTargetGender(_) => {
            profile.target_gender = Some(parse_gender(&t)?);
            let p = Profile::try_new(profile)
                .context("NewProfile isn't initialized")?;
            bot.send_message(
                dialogue.chat_id(),
                format!("(TODO: подтвердить да/нет)\n{:?}", p),
            )
            .await?;
            save_profile_to_db(&p).await?; // TODO: print on error
            remove_buttons!();
            dialogue.exit().await?;
        }
        _ => {}
    }
    Ok(())
}

async fn new_profile(
    bot: Bot,
    dialogue: MyDialogue,
    mut profile: NewProfile,
    state: State,
    msg: Message,
) -> anyhow::Result<()> {
    macro_rules! make_handler {
        (
            $text:ident,
            $retry_text:expr,
            $validate:expr,
            $action:expr,
            $next_text:expr,
            $next_state:expr
        ) => {
            match msg.text() {
                Some($text) if $validate => {
                    $action;
                    bot.send_message(msg.chat.id, $next_text).await?;
                    dialogue.update($next_state).await?;
                }
                _ => {
                    bot.send_message(msg.chat.id, $retry_text).await?;
                }
            }
        };
        (
            $text:ident,
            $retry_text:expr,
            $validate:expr,
            $action:expr,
            $next_text:expr,
            $keyboard:expr,
            $next_state:expr
        ) => {
            match msg.text() {
                Some($text) if $validate => {
                    $action;
                    bot.send_message(msg.chat.id, $next_text)
                        .reply_markup($keyboard)
                        .await?;
                    dialogue.update($next_state).await?;
                }
                _ => {
                    bot.send_message(msg.chat.id, $retry_text).await?;
                }
            }
        };
    }

    match state {
        State::NewName(_) => make_handler!(
            t,
            text::EDIT_NAME,
            (3..=30).contains(&t.len()),
            profile.name = Some(t.to_owned()),
            text::EDIT_GENDER,
            {
                let keyboard: Vec<Vec<InlineKeyboardButton>> = vec![
                    vec![InlineKeyboardButton::callback(
                        "Мужской",
                        Gender::Male.to_string(),
                    )],
                    vec![InlineKeyboardButton::callback(
                        "Женский",
                        Gender::Female.to_string(),
                    )],
                ];
                InlineKeyboardMarkup::new(keyboard)
            },
            State::NewGender(profile)
        ),
        State::NewAbout(_) => make_handler!(
            t,
            text::EDIT_ABOUT,
            (1..=100).contains(&t.len()),
            profile.about = Some(t.to_owned()),
            text::EDIT_TARGET_GENDER,
            {
                let keyboard: Vec<Vec<InlineKeyboardButton>> = vec![
                    vec![InlineKeyboardButton::callback(
                        "Мужской",
                        Gender::Male.to_string(),
                    )],
                    vec![InlineKeyboardButton::callback(
                        "Женский",
                        Gender::Female.to_string(),
                    )],
                ];
                InlineKeyboardMarkup::new(keyboard)
            },
            State::NewTargetGender(profile)
        ),
        _ => {}
    }

    Ok(())
}

#[derive(Debug, BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Доступные команды:")]
enum Command {
    #[command(description = "новая анкета")]
    NewProfile,
    #[command(description = "изменить анкету")]
    EditProfile,
    // #[command(description = "включить анкету")]
    // EnableAnketa,
    // #[command(description = "выключить анкета")]
    // DisableAnketa,
    Help,
}

mod text {
    pub const EDIT_NAME: &str =
        "Напиши имя от 3 до 20 символов (0 для пропуска).";
    pub const EDIT_GENDER: &str = "edit gender";
    pub const EDIT_GRADE: &str = "edit grade TODO";
    pub const EDIT_SUBJECT: &str = "Напиши предметы бота (0 для пропуска).";
    pub const EDIT_ABOUT: &str =
        "Напиши описание до 100 символов (0 для пропуска).";
    pub const EDIT_TARGET_GENDER: &str = "edit target gender";
}

// #[tracing::instrument(skip(db, bot))]
async fn answer(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    cmd: Command,
) -> anyhow::Result<()> {
    match cmd {
        Command::NewProfile => {
            dialogue.update(State::NewName(NewProfile::default())).await?;
            bot.send_message(msg.chat.id, text::EDIT_NAME).await?;
        }
        Command::EditProfile => {
            // if get_anketa(msg.chat.id.0).await?.is_some() {
            //     dialogue.update(State::NewName(NewProfile::default())).await?
            // ;     bot.send_message(msg.chat.id,
            // EDIT_NAME_TEXT).await?; } else {
            //     bot.send_message(msg.chat.id, "Сначала создайте анкету")
            //         .await?;
            // }
        }
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?;
        }
    }

    Ok(())
}

async fn invalid_command(bot: Bot, msg: Message) -> anyhow::Result<()> {
    bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?;
    Ok(())
}

async fn save_profile_to_db(_profile: &Profile) -> anyhow::Result<()> {
    Ok(())
}
