use itertools::Itertools;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

use crate::{text, Subjects};

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
