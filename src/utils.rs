use anyhow::bail;
use itertools::Itertools;
use teloxide::{
    requests::Requester,
    types::{
        ChatId, ChatKind, InlineKeyboardButton, InlineKeyboardMarkup, UserId,
    },
};

use crate::{
    text,
    types::{DatingPurpose, Subjects},
    Bot,
};

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
            first
                .name()
                .unwrap()
                .to_lowercase()
                .cmp(&other.name().unwrap().to_lowercase())
        })
        .map(|subject| {
            InlineKeyboardButton::callback(
                if selected.contains(subject) {
                    format!("✅ {}", subject.name().unwrap())
                } else {
                    subject.name().unwrap().to_owned()
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
                    format!("✅ {}", purpose.name().unwrap())
                } else {
                    purpose.name().unwrap().to_owned()
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

pub async fn user_url(bot: &Bot, id: i64) -> anyhow::Result<Option<url::Url>> {
    let ChatKind::Private(private) = bot.get_chat(ChatId(id)).await?.kind else {
        bail!("not private chat")
    };

    if let Some(username) = private.username {
        let mut url =
            url::Url::parse("tg://resolve").expect("tg url must be parsed");
        url.set_query(Some(&format!("domain={username}")));
        Ok(Some(url))
    } else if !has_privacy_enabled(bot, id).await? {
        let mut url =
            url::Url::parse("tg://user").expect("tg url must be parsed");
        url.set_query(Some(&format!("id={id}")));
        Ok(Some(url))
    } else {
        Ok(None)
    }
}

pub async fn has_privacy_enabled(bot: &Bot, user: i64) -> anyhow::Result<bool> {
    Ok(bot.get_chat(ChatId(user)).await?.has_private_forwards().is_some())
}

pub async fn check_user_subscribed_channel(
    bot: &Bot,
    user: i64,
) -> anyhow::Result<bool> {
    Ok(if let Ok(channel_id) = std::env::var("CHANNEL_ID") {
        let channel_id: i64 = channel_id.parse()?;
        let member = bot
            .get_chat_member(ChatId(channel_id), UserId(user as u64))
            .await?;
        member.is_present()
    } else {
        true
    })
}
