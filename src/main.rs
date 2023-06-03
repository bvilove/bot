use std::sync::Arc;

use anyhow::{anyhow, bail, Context, Result};
use bitflags::bitflags;
use chrono::Datelike;
use db::Database;
use entities::sea_orm_active_enums::Gender;
use itertools::Itertools;
use teloxide::{
    adaptors::{throttle::Limits, Throttle},
    dispatching::dialogue::InMemStorage,
    prelude::*,
    types::{
        Chat, ChatKind, InlineKeyboardButton, InlineKeyboardMarkup,
        KeyboardButton, KeyboardMarkup, KeyboardRemove,
    },
    utils::command::BotCommands,
};

mod db;

type Bot = Throttle<teloxide::Bot>;
type ProfileCreationDialogue =
    Dialogue<ProfileCreationState, InMemStorage<ProfileCreationState>>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    tracing::info!("Starting bot...");
    let bot = teloxide::Bot::from_env()
        .throttle(Limits { messages_per_min_chat: 30, ..Default::default() });

    let handler = dptree::entry()
        .enter_dialogue::<Update, InMemStorage<ProfileCreationState>, ProfileCreationState>()
        .branch(
            Update::filter_message()
                .branch(dptree::case![ProfileCreationState::SetName(a)].endpoint(handle_set_name))
                .branch(dptree::case![ProfileCreationState::SetGender(a)].endpoint(handle_set_gender))
                .branch(dptree::case![ProfileCreationState::SetPartnerGender(a)].endpoint(handle_set_partner_gender))
                .branch(dptree::case![ProfileCreationState::SetGraduationYear(a)].endpoint(handle_set_graduation_year))
                .branch(dptree::case![ProfileCreationState::SetAbout(a)].endpoint(handle_set_about))
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
                    dptree::case![ProfileCreationState::SetSubjects(a)]
                        .endpoint(handle_set_subjects_callback),
                )
                .branch(
                    dptree::case![ProfileCreationState::SetPartnerSubjects(a)]
                        .endpoint(handle_set_partner_subjects_callback),
                )
        );

    let database = db::Database::new().await?;

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![
            InMemStorage::<ProfileCreationState>::new(),
            Arc::new(database)
        ])
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
    graduation_year: Option<i16>,
    subjects: Option<Subjects>,
    partner_subjects: Option<Subjects>,
    about: Option<String>,
    target_gender: Option<Gender>,
}

#[derive(Debug)]
struct Profile {
    name: String,
    gender: Gender,
    graduation_year: i16,
    subjects: Subjects,
    partner_subjects: Subjects,
    about: String,
    partner_gender: Option<Gender>,
}

impl TryFrom<NewProfile> for Profile {
    type Error = anyhow::Error;

    fn try_from(new: NewProfile) -> Result<Self, Self::Error> {
        match new {
            NewProfile {
                name: Some(name),
                gender: Some(gender),
                graduation_year: Some(grade),
                subjects: Some(subjects),
                partner_subjects: Some(partner_subjects),
                about: Some(about),
                target_gender,
            } => Ok(Profile {
                name,
                gender,
                graduation_year: grade,
                subjects,
                partner_subjects,
                about,
                partner_gender: target_gender,
            }),
            _ => Err(anyhow!("can't create Profile from NewProfile")),
        }
    }
}

#[derive(Clone, Default)]
enum ProfileCreationState {
    #[default]
    Start,
    SetName(NewProfile),
    SetGender(NewProfile),
    SetPartnerGender(NewProfile),
    SetGraduationYear(NewProfile),
    SetSubjects(NewProfile),
    SetPartnerSubjects(NewProfile),
    SetAbout(NewProfile),
}

async fn request_set_name(bot: Bot, chat: Chat) -> Result<()> {
    match chat.kind {
        ChatKind::Public(_) => Err(anyhow!("chat isn't private")),
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

async fn handle_set_name(
    bot: Bot,
    dialogue: ProfileCreationDialogue,
    msg: Message,
    mut profile: NewProfile,
) -> Result<()> {
    match msg.text() {
        Some(text) if (3..=30).contains(&text.len()) => {
            profile.name = Some(text.to_owned());
            dialogue.update(ProfileCreationState::SetGender(profile)).await?;

            request_set_gender(bot, msg.chat).await?;
        }
        _ => {
            request_set_name(bot, msg.chat).await?;
        }
    }
    Ok(())
}

async fn request_set_gender(bot: Bot, chat: Chat) -> Result<()> {
    let keyboard = vec![vec![
        KeyboardButton::new("Мужской"),
        KeyboardButton::new("Женский"),
    ]];
    let keyboard_markup = KeyboardMarkup::new(keyboard).resize_keyboard(true);

    bot.send_message(chat.id, text::EDIT_GENDER)
        .reply_markup(keyboard_markup)
        .await?;
    Ok(())
}

async fn handle_set_gender(
    bot: Bot,
    dialogue: ProfileCreationDialogue,
    msg: Message,
    mut profile: NewProfile,
) -> Result<()> {
    let Some(text) = msg.text() else {bail!("no text in message")};
    let gender = match text {
        "Мужской" => Gender::Male,
        "Женский" => Gender::Female,
        &_ => {
            request_set_gender(bot, msg.chat).await?;
            return Ok(());
        }
    };

    profile.gender = Some(gender);
    dialogue.update(ProfileCreationState::SetPartnerGender(profile)).await?;

    request_set_partner_gender(bot, msg.chat).await?;

    Ok(())
}

async fn request_set_partner_gender(bot: Bot, chat: Chat) -> Result<()> {
    let keyboard = vec![
        vec![KeyboardButton::new("Парень"), KeyboardButton::new("Девушка")],
        vec![KeyboardButton::new("Не важно")],
    ];
    let keyboard_markup = KeyboardMarkup::new(keyboard).resize_keyboard(true);

    bot.send_message(chat.id, text::EDIT_PARTNER_GENDER)
        .reply_markup(keyboard_markup)
        .await?;
    Ok(())
}

async fn handle_set_partner_gender(
    bot: Bot,
    dialogue: ProfileCreationDialogue,
    msg: Message,
    mut profile: NewProfile,
) -> Result<()> {
    let Some(text) = msg.text() else {bail!("no text in message")};
    let gender = match text {
        "Парень" => Some(Gender::Male),
        "Девушка" => Some(Gender::Female),
        "Не важно" => None,
        &_ => {
            request_set_partner_gender(bot, msg.chat).await?;
            return Ok(());
        }
    };

    profile.target_gender = gender;
    dialogue.update(ProfileCreationState::SetGraduationYear(profile)).await?;

    request_set_graduation_year(bot, msg.chat).await?;

    Ok(())
}

async fn request_set_graduation_year(bot: Bot, chat: Chat) -> Result<()> {
    let keyboard =
        (6..=11).map(|n| KeyboardButton::new(n.to_string())).chunks(3);
    let keyboard_markup =
        KeyboardMarkup::new(keyboard.into_iter()).resize_keyboard(true);

    bot.send_message(chat.id, text::REQUEST_GRADE)
        .reply_markup(keyboard_markup)
        .await?;
    Ok(())
}

async fn handle_set_graduation_year(
    bot: Bot,
    dialogue: ProfileCreationDialogue,
    msg: Message,
    mut profile: NewProfile,
) -> Result<()> {
    let Some(text) = msg.text() else {bail!("no text in message")};
    let Ok(grade) = text.parse::<i32>() else {request_set_graduation_year(bot, msg.chat).await?; return Ok(())};

    let date = chrono::Local::now();

    let graduation_year = if date.month() < 9 {
        date.year() + (11 - grade)
    } else {
        date.year() + (11 - grade) + 1
    };

    profile.graduation_year = Some(graduation_year as i16);
    request_set_subjects(bot, msg.chat).await?;
    dialogue.update(ProfileCreationState::SetSubjects(profile)).await?;

    Ok(())
}

fn subject_name(subject: Subjects) -> Result<&'static str> {
    Ok(match subject {
        Subjects::Art => "Искусство 🎨",
        Subjects::Astronomy => "Астрономия 🌌",
        Subjects::Biology => "Биология 🔬",
        Subjects::Chemistry => "Химия 🧪",
        Subjects::Chinese => "Китайский 🇨🇳",
        Subjects::Ecology => "Экология ♻️",
        Subjects::Economics => "Экономика 💶",
        Subjects::English => "Английский 🇬🇧",
        Subjects::French => "Французский 🇫🇷",
        Subjects::Geography => "География 🌎",
        Subjects::German => "Немецкий 🇩🇪",
        Subjects::History => "История 📰",
        Subjects::Informatics => "Информатика 💻",
        Subjects::Italian => "Итальянский 🇮🇹",
        Subjects::Law => "Право 👨‍⚖️",
        Subjects::Literature => "Литература 📖",
        Subjects::Math => "Математика 📐",
        Subjects::Physics => "Физика ☢️",
        Subjects::Russian => "Русский 🇷🇺",
        Subjects::Safety => "ОБЖ 🪖",
        Subjects::Social => "Обществознание 👫",
        Subjects::Spanish => "Испанский 🇪🇸",
        Subjects::Sport => "Физкультура 🏐",
        Subjects::Technology => "Технология 🚜",
        _ => bail!("unknown subject"),
    })
}

// fn make_subjects_keyboard(selected: Subjects) -> InlineKeyboardMarkup {
//     let mut keyboard = Vec::new();

//     macro_rules! add_subjects {
//         ($type:expr, $subjects:expr) => {
//             keyboard.push(vec![InlineKeyboardButton::callback($type,
// $type)]);             keyboard.extend(
//                 $subjects
//                     .into_iter()
//                     .map(|s| {
//                         InlineKeyboardButton::callback(
//                             if selected.contains(s) {
//                                 format!("✅ {}", subject_name(s).unwrap())
//                             } else {
//                                 subject_name(s).unwrap().to_owned()
//                             },
//                             s.bits().to_string(),
//                         )
//                     })
//                     .chunks(3)
//                     .into_iter()
//                     .map(|r| r.collect()),
//             );
//         };
//     }

//     add_subjects!(text::SUBJECTS_HUMANITARIAN, [
//         Subjects::Art,
//         Subjects::Geography,
//         Subjects::History,
//         Subjects::Law,
//         Subjects::Literature,
//         Subjects::Social
//     ]);
//     add_subjects!(text::SUBJECTS_TECHNICAL, [
//         Subjects::Astronomy,
//         Subjects::Chemistry,
//         Subjects::Economics,
//         Subjects::Informatics,
//         Subjects::Math,
//         Subjects::Physics,
//     ]);
//     add_subjects!(text::SUBJECTS_LANGUAGES, [
//         Subjects::Chinese,
//         Subjects::English,
//         Subjects::French,
//         Subjects::German,
//         Subjects::Italian,
//         Subjects::Spanish
//     ]);
//     add_subjects!(text::SUBJECTS_OTHER, [
//         Subjects::Biology,
//         Subjects::Ecology,
//         Subjects::Russian,
//         Subjects::Safety,
//         Subjects::Sport,
//         Subjects::Technology,
//     ]);

//     keyboard.push(vec![InlineKeyboardButton::callback(
//         text::SUBJECTS_CONTINUE,
//         text::SUBJECTS_CONTINUE,
//     )]);
//     InlineKeyboardMarkup::new(keyboard)
// }

fn make_subjects_keyboard(selected: Subjects) -> InlineKeyboardMarkup {
    let mut keyboard: Vec<_> = Subjects::all()
        .iter_names()
        .chunks(3)
        .into_iter()
        .map(|row| {
            row.map(|(_, val)| {
                InlineKeyboardButton::callback(
                    if selected.contains(val) {
                        format!("✅ {}", subject_name(val).unwrap())
                    } else {
                        subject_name(val).unwrap().to_owned()
                    },
                    val.bits().to_string(),
                )
            })
            .collect()
        })
        .collect();

    keyboard.push(vec![InlineKeyboardButton::callback(
        text::SUBJECTS_CONTINUE,
        text::SUBJECTS_CONTINUE,
    )]);
    InlineKeyboardMarkup::new(keyboard)
}

async fn request_set_subjects(bot: Bot, chat: Chat) -> Result<()> {
    bot.send_message(chat.id, "* костыль для удаления клавиатуры *")
        .reply_markup(KeyboardRemove::new())
        .await?;
    bot.send_message(chat.id, text::EDIT_SUBJECTS)
        .reply_markup(make_subjects_keyboard(Subjects::default()))
        .await?;
    Ok(())
}

async fn handle_set_subjects_callback(
    bot: Bot,
    dialogue: ProfileCreationDialogue,
    mut profile: NewProfile,
    q: CallbackQuery,
) -> anyhow::Result<()> {
    let text = q.data.context("callback data not provided")?;
    let msg = q.message.context("callback without message")?;

    if text == text::SUBJECTS_CONTINUE {
        bot.edit_message_reply_markup(msg.chat.id, msg.id).await?;
        dialogue
            .update(ProfileCreationState::SetPartnerSubjects(profile))
            .await?;
        request_set_partner_subjects(bot, msg.chat).await?;
    } else {
        let subjects = profile.subjects.unwrap_or_default()
            ^ Subjects::from_bits(text.parse()?).context("subjects error")?;
        profile.subjects = Some(subjects);
        bot.edit_message_reply_markup(msg.chat.id, msg.id)
            .reply_markup(make_subjects_keyboard(subjects))
            .await?;
        dialogue.update(ProfileCreationState::SetSubjects(profile)).await?;
    }
    Ok(())
}

async fn request_set_partner_subjects(bot: Bot, chat: Chat) -> Result<()> {
    bot.send_message(chat.id, text::EDIT_PARTNER_SUBJECTS)
        .reply_markup(make_subjects_keyboard(Subjects::default()))
        .await?;
    Ok(())
}

async fn handle_set_partner_subjects_callback(
    bot: Bot,
    dialogue: ProfileCreationDialogue,
    mut profile: NewProfile,
    q: CallbackQuery,
) -> anyhow::Result<()> {
    let text = q.data.context("callback data not provided")?;
    let msg = q.message.context("callback without message")?;

    if text == text::SUBJECTS_CONTINUE {
        bot.edit_message_reply_markup(msg.chat.id, msg.id).await?;
        dialogue.update(ProfileCreationState::SetAbout(profile)).await?;
        request_set_about(bot, msg.chat).await?;
    } else {
        let subjects = profile.partner_subjects.unwrap_or_default()
            ^ Subjects::from_bits(text.parse()?).context("subjects error")?;
        profile.partner_subjects = Some(subjects);
        bot.edit_message_reply_markup(msg.chat.id, msg.id)
            .reply_markup(make_subjects_keyboard(subjects))
            .await?;
        dialogue
            .update(ProfileCreationState::SetPartnerSubjects(profile))
            .await?;
    }
    Ok(())
}

async fn request_set_about(bot: Bot, chat: Chat) -> Result<()> {
    bot.send_message(chat.id, text::EDIT_ABOUT).await?;
    Ok(())
}

async fn handle_set_about(
    db: Arc<Database>,
    bot: Bot,
    dialogue: ProfileCreationDialogue,
    msg: Message,
    mut profile: NewProfile,
) -> Result<()> {
    match msg.text() {
        Some(text) if (1..=1000).contains(&text.len()) => {
            dialogue.exit().await?;
            profile.about = Some(text.to_owned());
            let profile = Profile::try_from(profile)?;
            db.create_user(
                msg.chat.id.0,
                profile.name,
                profile.about,
                profile.gender,
                profile.partner_gender,
                profile.graduation_year,
                profile.subjects.0 .0,
                profile.partner_subjects.bits(),
            )
            .await?;
        }
        _ => {
            request_set_about(bot, msg.chat).await?;
        }
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
    pub const EDIT_NAME: &str = "Укажите ваше имя (3-20 символов)";
    pub const EDIT_GENDER: &str = "Выберите ваш пол";
    pub const REQUEST_GRADE: &str = "В каком вы сейчас классе?";
    pub const EDIT_SUBJECTS: &str = "Какие предметы вы ботаете? Нажмите на \
                                     предмет, чтобы добавить или убрать его.";
    pub const EDIT_PARTNER_SUBJECTS: &str =
        "Какие предметы должен ботать тот, кого вы ищете? Нажмите на предмет, \
         чтобы добавить или убрать его. Достаточно одного совпадения. Если \
         вам не важно, что он ботает, не выбирайте ничего.";
    pub const EDIT_ABOUT: &str = "Немного расскажите о себе";
    pub const EDIT_PARTNER_GENDER: &str = "Кого вы ищете?";
    pub const SUBJECTS_CONTINUE: &str = "Продолжить";
    // pub const SUBJECTS_HUMANITARIAN: &str = "Гуманитарные";
    // pub const SUBJECTS_TECHNICAL: &str = "Технические";
    // pub const SUBJECTS_LANGUAGES: &str = "Языковые";
    // pub const SUBJECTS_OTHER: &str = "Другие";
}

// #[tracing::instrument(skip(db, bot))]
async fn answer(
    bot: Bot,
    dialogue: ProfileCreationDialogue,
    msg: Message,
    cmd: Command,
) -> anyhow::Result<()> {
    match cmd {
        Command::NewProfile => {
            dialogue
                .update(ProfileCreationState::SetName(NewProfile::default()))
                .await?;
            request_set_name(bot, msg.chat).await?;
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
