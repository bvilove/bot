use std::sync::Arc;

use anyhow::Context;
use chrono::Datelike;
use db::Database;
use entities::sea_orm_active_enums::Gender;
use teloxide::{
    prelude::*,
    types::{Chat, KeyboardRemove},
};

use crate::{
    db, text, utils, Bot, MyDialogue, NewProfile, Profile, State, Subjects,
};

async fn next_state(
    dialogue: MyDialogue,
    state: &State,
    p: NewProfile,
) -> anyhow::Result<()> {
    use State::*;
    let next_state = match state {
        SetName(_) => SetGender(p),
        SetGender(_) => SetPartnerGender(p),
        SetPartnerGender(_) => SetGraduationYear(p),
        SetGraduationYear(_) => SetSubjects(p),
        SetSubjects(_) => SetPartnerSubjects(p),
        SetPartnerSubjects(_) => SetCity(p),
        SetCity(_) => SetPartnerCity(p),
        SetPartnerCity(_) => SetAbout(p),
        _ => {
            dialogue.exit().await?;
            anyhow::bail!("wrong state: {:?}", state)
        }
    };
    dialogue.update(next_state).await?;
    Ok(())
}

async fn print_current_state(
    state: &State,
    bot: Bot,
    chat: Chat,
) -> anyhow::Result<()> {
    use State::*;

    use crate::request::*;
    match state {
        SetName(_) => request_set_name(bot, chat).await?,
        SetGender(_) => request_set_gender(bot, chat).await?,
        SetPartnerGender(_) => request_set_partner_gender(bot, chat).await?,
        SetGraduationYear(_) => request_set_graduation_year(bot, chat).await?,
        SetSubjects(_) => request_set_subjects(bot, chat).await?,
        SetPartnerSubjects(_) => {
            request_set_partner_subjects(bot, chat).await?
        }
        SetCity(_) => request_set_city(bot, chat).await?,
        SetPartnerCity(_) => request_set_partner_city(bot, chat).await?,
        SetAbout(_) => request_set_about(bot, chat).await?,
        _ => anyhow::bail!("wrong state: {:?}", state),
    };
    Ok(())
}

async fn print_next_state(
    state: &State,
    bot: Bot,
    chat: Chat,
) -> anyhow::Result<()> {
    use State::*;

    use crate::request::*;
    match state {
        SetName(_) => request_set_gender(bot, chat).await?,
        SetGender(_) => request_set_partner_gender(bot, chat).await?,
        SetPartnerGender(_) => request_set_graduation_year(bot, chat).await?,
        SetGraduationYear(_) => request_set_subjects(bot, chat).await?,
        SetSubjects(_) => request_set_partner_subjects(bot, chat).await?,
        SetPartnerSubjects(_) => request_set_city(bot, chat).await?,
        SetCity(_) => request_set_partner_city(bot, chat).await?,
        _ => anyhow::bail!("wrong state: {:?}", state),
    };
    Ok(())
}

pub async fn handle_set_city(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    mut profile: NewProfile,
    state: State,
) -> anyhow::Result<()> {
    // let city = TODO
    profile.city = Some(1);
    next_state(dialogue, &state, profile).await?;
    print_next_state(&state, bot, msg.chat).await?;

    Ok(())
}

pub async fn handle_set_partner_city(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    mut profile: NewProfile,
    state: State,
) -> anyhow::Result<()> {
    let same_partner_city = match msg.text().context("no text in message")? {
        text::USER_CITY_CURRENT => true,
        text::USER_CITY_ANY => false,
        _ => {
            print_current_state(&state, bot, msg.chat).await?;
            return Ok(());
        }
    };

    profile.same_partner_city = Some(same_partner_city);
    next_state(dialogue, &state, profile).await?;
    print_next_state(&state, bot, msg.chat).await?;

    Ok(())
}

pub async fn handle_set_name(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    mut profile: NewProfile,
    state: State,
) -> anyhow::Result<()> {
    match msg.text() {
        Some(text) if (3..=30).contains(&text.len()) => {
            profile.name = Some(text.to_owned());
            next_state(dialogue, &state, profile).await?;

            bot.send_message(
                msg.chat.id,
                format!(
                    "Выбранное имя: {text}.\nЕго можно будет изменить позже \
                     командой /setname"
                ),
            )
            .await?;
            print_next_state(&state, bot, msg.chat).await?;
        }
        _ => {
            print_current_state(&state, bot, msg.chat).await?;
        }
    }
    Ok(())
}

pub async fn handle_set_gender(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    mut profile: NewProfile,
    state: State,
) -> anyhow::Result<()> {
    let gender = match msg.text().context("no text in message")? {
        text::USER_GENDER_MALE => Gender::Male,
        text::USER_GENDER_FEMALE => Gender::Female,
        _ => {
            print_current_state(&state, bot, msg.chat).await?;
            return Ok(());
        }
    };

    profile.gender = Some(gender);
    next_state(dialogue, &state, profile).await?;
    print_next_state(&state, bot, msg.chat).await?;

    Ok(())
}

pub async fn handle_set_partner_gender(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    mut profile: NewProfile,
    state: State,
) -> anyhow::Result<()> {
    let gender = match msg.text().context("no text in message")? {
        text::PARTNER_GENDER_MALE => Some(Gender::Male),
        text::PARTNER_GENDER_FEMALE => Some(Gender::Female),
        text::PARTNER_GENDER_ALL => None,
        _ => {
            print_current_state(&state, bot, msg.chat).await?;
            return Ok(());
        }
    };

    profile.partner_gender = gender;
    next_state(dialogue, &state, profile).await?;
    print_next_state(&state, bot, msg.chat).await?;

    Ok(())
}

pub async fn handle_set_graduation_year(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    mut profile: NewProfile,
    state: State,
) -> anyhow::Result<()> {
    let Ok(grade) = msg
        .text()
        .context("no text in message")?
        .parse::<i32>()
    else {
        print_current_state(&state, bot, msg.chat).await?;
        return Ok(())
    };

    let date = chrono::Local::now();

    let graduation_year = if date.month() < 9 {
        date.year() + (11 - grade)
    } else {
        date.year() + (11 - grade) + 1
    };

    profile.graduation_year = Some(graduation_year as i16);
    next_state(dialogue, &state, profile).await?;

    bot.send_message(
        msg.chat.id,
        format!(
            "Хорошо, сейчас вы в {grade} классе и закончите школу в \
             {graduation_year} году.\nИзменить это можно командой /setgrade"
        ),
    )
    .reply_markup(KeyboardRemove::new())
    .await?;
    print_next_state(&state, bot, msg.chat).await?;

    Ok(())
}

pub async fn handle_set_subjects_callback(
    bot: Bot,
    dialogue: MyDialogue,
    mut profile: NewProfile,
    state: State,
    q: CallbackQuery,
) -> anyhow::Result<()> {
    let text = q.data.context("callback data not provided")?;
    let msg = q.message.context("callback without message")?;

    if text == text::SUBJECTS_CONTINUE || text == text::SUBJECTS_USER_EMPTY {
        profile.subjects = Some(profile.subjects.unwrap_or_default());

        bot.edit_message_reply_markup(msg.chat.id, msg.id).await?;

        let user_subjects =
            if profile.subjects.context("subjects must be set")?.is_empty() {
                "Вы ничего не ботаете.".to_owned()
            } else {
                format!(
                    "Предметы, которые вы ботаете: {}.",
                    utils::subjects_list(
                        profile.subjects.context("subjects must be set")?,
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

        next_state(dialogue, &state, profile).await?;
        print_next_state(&state, bot, msg.chat).await?;
    } else {
        let subjects = profile.subjects.unwrap_or_default()
            ^ Subjects::from_bits(text.parse()?).context("subjects error")?;
        profile.subjects = Some(subjects);
        bot.edit_message_reply_markup(msg.chat.id, msg.id)
            .reply_markup(utils::make_subjects_keyboard(
                subjects,
                utils::SubjectsKeyboardType::User,
            ))
            .await?;
        dialogue.update(State::SetSubjects(profile)).await?;
    }
    Ok(())
}

pub async fn handle_set_partner_subjects_callback(
    bot: Bot,
    dialogue: MyDialogue,
    mut profile: NewProfile,
    state: State,
    q: CallbackQuery,
) -> anyhow::Result<()> {
    let text = q.data.context("callback data not provided")?;
    let msg = q.message.context("callback without message")?;

    if text == text::SUBJECTS_CONTINUE || text == text::SUBJECTS_PARTNER_EMPTY {
        profile.partner_subjects =
            Some(profile.partner_subjects.unwrap_or_default());

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
                utils::subjects_list(
                    profile.partner_subjects.context("subjects must be set")?,
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

        next_state(dialogue, &state, profile).await?;
        print_next_state(&state, bot, msg.chat).await?;
    } else {
        let subjects = profile.partner_subjects.unwrap_or_default()
            ^ Subjects::from_bits(text.parse()?).context("subjects error")?;
        profile.partner_subjects = Some(subjects);
        bot.edit_message_reply_markup(msg.chat.id, msg.id)
            .reply_markup(utils::make_subjects_keyboard(
                subjects,
                utils::SubjectsKeyboardType::Partner,
            ))
            .await?;
        dialogue.update(State::SetPartnerSubjects(profile)).await?;
    }
    Ok(())
}

pub async fn handle_set_about(
    db: Arc<Database>,
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    mut profile: NewProfile,
    state: State,
) -> anyhow::Result<()> {
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
                profile.city,
                profile.same_partner_city,
            )
            .await?;
        }
        _ => {
            print_current_state(&state, bot, msg.chat).await?;
        }
    }
    Ok(())
}
