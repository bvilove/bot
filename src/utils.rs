use anyhow::{bail, Context};
use chrono::Datelike;
use itertools::Itertools;
use teloxide::{
    requests::Requester,
    types::{ChatId, ChatKind, InlineKeyboardButton, InlineKeyboardMarkup},
};

use crate::{text, Bot, DatingPurpose, Subjects};

fn dating_purpose_name(purpose: DatingPurpose) -> anyhow::Result<&'static str> {
    Ok(match purpose {
        DatingPurpose::Friendship => "Дружба 🧑‍🤝‍🧑",
        DatingPurpose::Studies => "Учёба 📚",
        DatingPurpose::Relationship => "Отношения 💕",
        _ => anyhow::bail!("unknown subject"),
    })
}

fn subject_name(subject: Subjects) -> anyhow::Result<&'static str> {
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
        _ => anyhow::bail!("unknown subject"),
    })
}

pub fn subjects_list(subjects: Subjects) -> anyhow::Result<String> {
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

pub fn dating_purpose_list(purpose: DatingPurpose) -> anyhow::Result<String> {
    Ok(DatingPurpose::all()
        .into_iter()
        .filter(|s| purpose.contains(*s))
        .map(|s| dating_purpose_name(s).unwrap())
        .enumerate()
        .map(|(i, s)| if i != 0 { format!(", {}", s) } else { s.to_owned() })
        .collect())
}

pub enum SubjectsKeyboardType {
    User,
    Partner,
}

pub fn make_subjects_keyboard(
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
                format!(
                    "{}{}",
                    match tp {
                        SubjectsKeyboardType::Partner => "d",
                        SubjectsKeyboardType::User => "s",
                    },
                    subject.bits().to_string()
                ),
            )
        })
        .chunks(3)
        .into_iter()
        .map(|row| row.collect())
        .collect();

    let (text, cont) = match tp {
        SubjectsKeyboardType::Partner => (
            if selected.is_empty() {
                text::SUBJECTS_PARTNER_EMPTY
            } else {
                text::SUBJECTS_CONTINUE
            },
            "dcontinue",
        ),
        SubjectsKeyboardType::User => (
            if selected.is_empty() {
                text::SUBJECTS_USER_EMPTY
            } else {
                text::SUBJECTS_CONTINUE
            },
            "scontinue",
        ),
    };
    keyboard.push(vec![InlineKeyboardButton::callback(text, cont)]);
    InlineKeyboardMarkup::new(keyboard)
}

pub fn make_dating_purpose_keyboard(
    selected: DatingPurpose,
) -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<_>> = DatingPurpose::all()
        .into_iter()
        .map(|purpose| {
            InlineKeyboardButton::callback(
                if selected.contains(purpose) {
                    format!("✅ {}", dating_purpose_name(purpose).unwrap())
                } else {
                    dating_purpose_name(purpose).unwrap().to_owned()
                },
                format!("p{}", purpose.bits().to_string()),
            )
        })
        .chunks(3)
        .into_iter()
        .map(|row| row.collect())
        .collect();

    if selected != DatingPurpose::empty() {
        keyboard.push(vec![InlineKeyboardButton::callback(
            "Продолжить",
            "pcontinue",
        )]);
    }
    InlineKeyboardMarkup::new(keyboard)
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

pub fn graduation_year_from_grade(grade: i32) -> anyhow::Result<i32> {
    let date = chrono::Local::now();

    let year = if date.month() < 9 {
        date.year() + (11 - grade)
    } else {
        date.year() + (11 - grade) + 1
    };

    Ok(year)
}

pub fn grade_from_graduation_year(graduation_year: i32) -> anyhow::Result<i32> {
    let date = chrono::Local::now();

    let year = if date.month() < 9 {
        11 - (graduation_year - date.year())
    } else {
        11 - (graduation_year - date.year()) + 1
    };

    Ok(year)
}

pub async fn user_url(bot: &Bot, id: i64) -> anyhow::Result<url::Url> {
    if has_privacy_settings(bot, id).await? {
        let mut url =
            url::Url::parse("tg://user").expect("tg url must be parsed");
        url.set_query(Some(&format!("id={id}")));
        Ok(url)
    } else {
        let mut url =
            url::Url::parse("tg://resolve").expect("tg url must be parsed");
        let ChatKind::Private(private) = bot.get_chat(ChatId(id)).await?.kind else {
            bail!("not private chat")
        };
        let username = private.username.context("username must be set")?;
        url.set_query(Some(&format!("domain={username}")));
        Ok(url)
    }
}

pub async fn has_privacy_settings(
    bot: &Bot,
    user: i64,
) -> anyhow::Result<bool> {
    Ok(bot.get_chat(ChatId(user)).await?.has_private_forwards().is_none())
}
