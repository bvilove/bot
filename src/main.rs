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
type ProfileCreationDialogue = Dialogue<State, InMemStorage<State>>;

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
                .branch(
                    dptree::case![State::SetName(a)].endpoint(handle_set_name),
                )
                .branch(
                    dptree::case![State::SetGender(a)]
                        .endpoint(handle_set_gender),
                )
                .branch(
                    dptree::case![State::SetPartnerGender(a)]
                        .endpoint(handle_set_partner_gender),
                )
                .branch(
                    dptree::case![State::SetGraduationYear(a)]
                        .endpoint(handle_set_graduation_year),
                )
                .branch(
                    dptree::case![State::SetAbout(a)]
                        .endpoint(handle_set_about),
                )
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
                    dptree::case![State::SetSubjects(a)]
                        .endpoint(handle_set_subjects_callback),
                )
                .branch(
                    dptree::case![State::SetPartnerSubjects(a)]
                        .endpoint(handle_set_partner_subjects_callback),
                ),
        );

    let database = db::Database::new().await?;

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![
            InMemStorage::<State>::new(),
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
enum State {
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
            dialogue.update(State::SetGender(profile)).await?;

            bot.send_message(
                msg.chat.id,
                format!(
                    "Выбранное имя: {text}.\nЕго можно будет изменить позже \
                     командой /setname"
                ),
            )
            .await?;
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
        KeyboardButton::new(text::USER_GENDER_MALE),
        KeyboardButton::new(text::USER_GENDER_FEMALE),
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
        text::USER_GENDER_MALE => Gender::Male,
        text::USER_GENDER_FEMALE => Gender::Female,
        &_ => {
            request_set_gender(bot, msg.chat).await?;
            return Ok(());
        }
    };

    profile.gender = Some(gender);
    dialogue.update(State::SetPartnerGender(profile)).await?;

    request_set_partner_gender(bot, msg.chat).await?;

    Ok(())
}

async fn request_set_partner_gender(bot: Bot, chat: Chat) -> Result<()> {
    let keyboard = vec![
        vec![
            KeyboardButton::new(text::PARTNER_GENDER_MALE),
            KeyboardButton::new(text::PARTNER_GENDER_FEMALE),
        ],
        vec![KeyboardButton::new(text::PARTNER_GENDER_ALL)],
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
        text::PARTNER_GENDER_MALE => Some(Gender::Male),
        text::PARTNER_GENDER_FEMALE => Some(Gender::Female),
        text::PARTNER_GENDER_ALL => None,
        &_ => {
            request_set_partner_gender(bot, msg.chat).await?;
            return Ok(());
        }
    };

    profile.target_gender = gender;
    dialogue.update(State::SetGraduationYear(profile)).await?;

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
    dialogue.update(State::SetSubjects(profile)).await?;

    bot.send_message(
        msg.chat.id,
        format!(
            "Хорошо, сейчас вы в {grade} классе и закончите школу в \
             {graduation_year} году.\nИзменить это можно командой /setgrade"
        ),
    )
    .reply_markup(KeyboardRemove::new())
    .await?;
    request_set_subjects(bot, msg.chat).await?;

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

fn subjects_list(subjects: Subjects) -> Result<String> {
    Ok(Subjects::all()
        .into_iter()
        .filter(|s| subjects.contains(*s))
        .map(|s| subject_name(s).unwrap())
        .sorted_by(|first, other| {
            first.to_lowercase().cmp(&other.to_lowercase())
        })
        .enumerate()
        .map(|(i, s)| if i != 0 { format!(", {}", s) } else { s.to_owned() })
        .collect())
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

enum SubjectsKeyboardType {
    User,
    Partner,
}

fn make_subjects_keyboard(
    selected: Subjects,
    tp: SubjectsKeyboardType,
) -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<_>> = Subjects::all()
        .into_iter()
        .sorted_by(|first, other| {
            subject_name(*first)
                .unwrap()
                .to_lowercase()
                .cmp(&subject_name(*other).unwrap().to_lowercase())
        })
        .map(|subject| {
            InlineKeyboardButton::callback(
                if selected.contains(subject) {
                    format!("✅ {}", subject_name(subject).unwrap())
                } else {
                    subject_name(subject).unwrap().to_owned()
                },
                subject.bits().to_string(),
            )
        })
        .chunks(3)
        .into_iter()
        .map(|row| row.collect())
        .collect();

    let text = match tp {
        SubjectsKeyboardType::Partner => {
            if selected.is_empty() {
                text::SUBJECTS_PARTNER_EMPTY
            } else {
                text::SUBJECTS_CONTINUE
            }
        }
        SubjectsKeyboardType::User => {
            if selected.is_empty() {
                text::SUBJECTS_USER_EMPTY
            } else {
                text::SUBJECTS_CONTINUE
            }
        }
    };
    keyboard.push(vec![InlineKeyboardButton::callback(text, text)]);
    InlineKeyboardMarkup::new(keyboard)
}

async fn request_set_subjects(bot: Bot, chat: Chat) -> Result<()> {
    bot.send_message(chat.id, text::EDIT_SUBJECTS)
        .reply_markup(make_subjects_keyboard(
            Subjects::default(),
            SubjectsKeyboardType::User,
        ))
        .await?;
    Ok(())
}

async fn handle_set_subjects_callback(
    bot: Bot,
    dialogue: ProfileCreationDialogue,
    mut profile: NewProfile,
    q: CallbackQuery,
) -> Result<()> {
    let text = q.data.context("callback data not provided")?;
    let msg = q.message.context("callback without message")?;

    if text == text::SUBJECTS_CONTINUE || text == text::SUBJECTS_USER_EMPTY {
        profile.subjects =
            Some(profile.subjects.unwrap_or_else(|| Subjects::empty()));

        bot.edit_message_reply_markup(msg.chat.id, msg.id).await?;

        let user_subjects = if profile
            .subjects
            .context("subjects must be set")?
            .is_empty()
        {
            "Вы ничего не ботаете.".to_owned()
        } else {
            format!(
                "Предметы, которые вы ботаете: {}.",
                subjects_list(
                    profile.subjects.clone().context("subjects must be set")?,
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

        request_set_partner_subjects(bot, msg.chat).await?;

        dialogue.update(State::SetPartnerSubjects(profile)).await?;
    } else {
        let subjects = profile.subjects.unwrap_or_default()
            ^ Subjects::from_bits(text.parse()?).context("subjects error")?;
        profile.subjects = Some(subjects);
        bot.edit_message_reply_markup(msg.chat.id, msg.id)
            .reply_markup(make_subjects_keyboard(
                subjects,
                SubjectsKeyboardType::User,
            ))
            .await?;
        dialogue.update(State::SetSubjects(profile)).await?;
    }
    Ok(())
}

async fn request_set_partner_subjects(bot: Bot, chat: Chat) -> Result<()> {
    bot.send_message(chat.id, text::EDIT_PARTNER_SUBJECTS)
        .reply_markup(make_subjects_keyboard(
            Subjects::default(),
            SubjectsKeyboardType::Partner,
        ))
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

    if text == text::SUBJECTS_CONTINUE || text == text::SUBJECTS_PARTNER_EMPTY {
        profile.partner_subjects =
            Some(profile.partner_subjects.unwrap_or_else(|| Subjects::empty()));

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
                subjects_list(
                    profile
                        .partner_subjects
                        .clone()
                        .context("subjects must be set")?,
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

        request_set_about(bot, msg.chat).await?;

        dialogue.update(State::SetAbout(profile)).await?;
    } else {
        let subjects = profile.partner_subjects.unwrap_or_default()
            ^ Subjects::from_bits(text.parse()?).context("subjects error")?;
        profile.partner_subjects = Some(subjects);
        bot.edit_message_reply_markup(msg.chat.id, msg.id)
            .reply_markup(make_subjects_keyboard(
                subjects,
                SubjectsKeyboardType::Partner,
            ))
            .await?;
        dialogue.update(State::SetPartnerSubjects(profile)).await?;
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
    pub const EDIT_NAME: &str = "Как вас зовут?";
    pub const EDIT_GENDER: &str = "Теперь выберите ваш пол";
    pub const USER_GENDER_MALE: &str = "Я парень";
    pub const USER_GENDER_FEMALE: &str = "Я девушка";
    pub const EDIT_PARTNER_GENDER: &str = "Кого вы ищете?";
    pub const PARTNER_GENDER_MALE: &str = "Парня";
    pub const PARTNER_GENDER_FEMALE: &str = "Девушку";
    pub const PARTNER_GENDER_ALL: &str = "Не важно";
    pub const REQUEST_GRADE: &str = "В каком вы сейчас классе?";
    pub const EDIT_SUBJECTS: &str = "Какие предметы вы ботаете? Нажмите на \
                                     предмет, чтобы добавить или убрать его.";
    pub const EDIT_PARTNER_SUBJECTS: &str =
        "Выберите предметы, хотя бы один из которых должен ботать тот, кого \
         вы ищете. Нажмите на предмет, чтобы добавить или убрать его.";
    pub const EDIT_ABOUT: &str = "Немного расскажите о себе";
    pub const SUBJECTS_CONTINUE: &str = "Продолжить";
    pub const SUBJECTS_PARTNER_EMPTY: &str = "Не важно";
    pub const SUBJECTS_USER_EMPTY: &str = "Никакие";
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
            dialogue.update(State::SetName(NewProfile::default())).await?;
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
