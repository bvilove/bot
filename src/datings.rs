use std::sync::Arc;

use anyhow::{bail, Context};
use entities::{datings, sea_orm_active_enums::ImageKind};
use teloxide::{
    prelude::*,
    types::{
        Chat, InlineKeyboardButton, InlineKeyboardMarkup, InputFile,
        InputMedia, InputMediaPhoto, InputMediaVideo, KeyboardButton,
        KeyboardMarkup, KeyboardRemove, MessageId,
    },
    ApiError, RequestError,
};
use tracing::*;

use crate::{
    db::Database, text, types::PublicProfile, Bot, EditProfile, MyDialogue,
};

pub async fn send_profile(
    bot: &Bot,
    db: &Arc<Database>,
    id: i64,
) -> anyhow::Result<()> {
    let user =
        db.get_user(id).await?.context("user to send profile not found")?;

    let profile: PublicProfile = (&user).try_into()?;

    send_user_photos(bot, db, id, id).await?;

    let msg = format!("Так выглядит ваша анкета:\n\n{profile}");

    bot.send_message(ChatId(id), msg)
        .reply_markup(KeyboardRemove::new())
        .await?;

    send_ready_to_datings(bot, id).await?;

    Ok(())
}

async fn send_ready_to_datings(bot: &Bot, id: i64) -> anyhow::Result<()> {
    let keyboard =
        vec![vec![InlineKeyboardButton::callback("Смотреть анкеты 🚀", "🚀")]];
    let keyboard_markup = InlineKeyboardMarkup::new(keyboard);

    bot.send_message(ChatId(id), text::READY_FOR_DATINGS)
        .reply_markup(keyboard_markup)
        .await?;
    Ok(())
}

pub async fn send_recommendation(
    bot: &Bot,
    db: &Arc<Database>,
    chat: ChatId,
) -> anyhow::Result<()> {
    if !crate::utils::check_user_subscribed_channel(bot, chat.0).await? {
        let keyboard = vec![vec![InlineKeyboardButton::callback(
            "Я подписался на канал",
            "🚀",
        )]];
        let keyboard_markup = InlineKeyboardMarkup::new(keyboard);
        bot.send_message(
            chat,
            "Пожалуйста, подпишитесь на наш канал https://t.me/bvilove",
        )
        .reply_markup(keyboard_markup)
        .await?;
        return Ok(());
    };

    if crate::utils::user_url(bot, chat.0).await?.is_none() {
        let keyboard = vec![vec![InlineKeyboardButton::callback(
            "Я сделал юзернейм",
            "🚀",
        )]];
        let keyboard_markup = InlineKeyboardMarkup::new(keyboard);
        bot.send_message(chat, text::PLEASE_ALLOW_FORWARDING)
            .reply_markup(keyboard_markup)
            .await?;
        return Ok(());
    }

    match db.get_partner(chat.0).await? {
        Some((dating, partner)) => {
            // Clean buttons of old message with this dating if it exist
            if let Some(msg) = dating.initiator_msg_id {
                if let Err(e) = bot
                    .edit_message_reply_markup(
                        ChatId(dating.initiator_id),
                        MessageId(msg),
                    )
                    .await
                {
                    sentry_anyhow::capture_anyhow(
                        &anyhow::Error::from(e)
                            .context("error while editing old message"),
                    );
                }
            }

            send_user_photos(bot, db, partner.id, chat.0).await?;

            let keyboard = vec![vec![
                InlineKeyboardButton::callback(
                    "👎",
                    format!("👎{}", dating.id),
                ),
                InlineKeyboardButton::callback(
                    "💌",
                    format!("💌{}", dating.id),
                ),
                InlineKeyboardButton::callback(
                    "👍",
                    format!("👍{}", dating.id),
                ),
            ]];
            let keyboard_markup = InlineKeyboardMarkup::new(keyboard);

            let partner_profile: PublicProfile = (&partner).try_into()?;

            let sent_msg = bot
                .send_message(chat, partner_profile.to_string())
                .reply_markup(keyboard_markup)
                .await?;

            db.set_dating_initiator_msg(dating.id, sent_msg.id.0).await?;
        }
        None => {
            let keyboard = vec![vec![InlineKeyboardButton::callback(
                "Попробовать ещё раз",
                "🚀",
            )]];
            let keyboard_markup = InlineKeyboardMarkup::new(keyboard);
            bot.send_message(chat, text::PARTNER_NOT_FOUND)
                .reply_markup(keyboard_markup)
                .await?;
        }
    }

    // if partner_images.is_empty() {
    //     bot.send_message(chat.id,
    // partner_msg).reply_markup(keyboard_markup).await?; } else {
    //     let medias =
    //         partner_images.into_iter().enumerate().map(|(index, id)| {
    //             let input_file = InputFile::file_id(id);
    //             let mut input_media_photo = InputMediaPhoto::new(input_file);
    //             if index == 0 {
    //                 input_media_photo =
    // input_media_photo.caption(&partner_msg)             }
    //             InputMedia::Photo(input_media_photo)
    //         });
    //     bot.send_media_group(chat.id, medias).await?;
    // }

    Ok(())
}

pub async fn send_like(
    db: &Arc<Database>,
    bot: &Bot,
    dating: &entities::datings::Model,
    msg: Option<String>,
) -> anyhow::Result<()> {
    let user = db
        .get_user(dating.initiator_id)
        .await?
        .context("dating initiator not found")?;

    let user_profile: PublicProfile = (&user).try_into()?;

    let like_msg = match msg {
        Some(m) => {
            format!(
                "Кому то понравилась ваша анкета и он оставил вам \
                 сообщение:\n{m}\n\n{user_profile}"
            )
        }
        None => format!("Кому то понравилась ваша анкета:\n\n{user_profile}"),
    };

    match send_user_photos(bot, db, dating.initiator_id, dating.partner_id)
        .await
    {
        Err(crate::AppError::Telegram(RequestError::Api(
            ApiError::BotBlocked,
        ))) => {
            warn!("bot was blocked");
            db.create_or_update_user(EditProfile {
                active: Some(false),
                ..EditProfile::new(dating.partner_id)
            })
            .await?;
            return Ok(());
        }
        Err(e) => {
            sentry_anyhow::capture_anyhow(
                &anyhow::Error::from(e)
                    .context("error sending like while sending user photos"),
            );
        }
        Ok(_) => {}
    }

    let keyboard = vec![vec![
        InlineKeyboardButton::callback("💔", format!("💔{}", dating.id)),
        InlineKeyboardButton::callback("❤", format!("❤{}", dating.id)),
    ]];
    let keyboard_markup = InlineKeyboardMarkup::new(keyboard);

    match bot
        .send_message(ChatId(dating.partner_id), like_msg)
        .reply_markup(keyboard_markup)
        .await
    {
        Err(RequestError::Api(ApiError::BotBlocked)) => {
            warn!("bot was blocked");
            db.create_or_update_user(EditProfile {
                active: Some(false),
                ..EditProfile::new(dating.partner_id)
            })
            .await?;
            return Ok(());
        }
        Err(e) => {
            sentry_anyhow::capture_anyhow(
                &anyhow::Error::from(e)
                    .context("error sending like while sending profile"),
            );
        }
        Ok(_) => {}
    }

    Ok(())
}

pub async fn mutual_like(
    bot: &Bot,
    db: &Arc<Database>,
    dating: &datings::Model,
) -> anyhow::Result<()> {
    let partner = db
        .get_user(dating.partner_id)
        .await?
        .context("dating partner not found")?;

    let partner_profile: PublicProfile = (&partner).try_into()?;

    db.set_dating_partner_reaction(dating.id, true).await?;

    if let Err(e) =
        send_user_photos(bot, db, dating.partner_id, dating.initiator_id).await
    {
        sentry_anyhow::capture_anyhow(
            &anyhow::Error::from(e)
                .context("error sending user photos to dating initiator"),
        );
    };

    let initiator_keyboard = vec![vec![InlineKeyboardButton::url(
        "Открыть чат",
        crate::utils::user_url(bot, partner.id)
            .await?
            .context("can't get url")?,
    )]];
    let initiator_keyboard_markup =
        InlineKeyboardMarkup::new(initiator_keyboard);
    let initiator_msg = format!("Взаимный лайк!\n\n{partner_profile}");
    if let Err(e) = bot
        .send_message(ChatId(dating.initiator_id), initiator_msg)
        .reply_markup(initiator_keyboard_markup)
        .await
    {
        sentry_anyhow::capture_anyhow(
            &anyhow::Error::from(e)
                .context("error sending profile to dating initiator"),
        );
    }

    Ok(())
}

async fn send_user_photos(
    bot: &Bot,
    db: &Arc<Database>,
    user: i64,
    chat: i64,
) -> std::result::Result<(), crate::AppError> {
    let user_images = db.get_images(user).await?;

    if !user_images.is_empty() {
        let medias = user_images.into_iter().map(|(id, kind)| {
            let input_file = InputFile::file_id(id);
            match kind {
                ImageKind::Image => {
                    let input_media_photo = InputMediaPhoto::new(input_file);
                    InputMedia::Photo(input_media_photo)
                }
                ImageKind::Video => {
                    let input_media_video = InputMediaVideo::new(input_file);
                    InputMedia::Video(input_media_video)
                }
            }
        });
        bot.send_media_group(ChatId(chat), medias).await?;
    }
    Ok(())
}

pub async fn request_like_msg(bot: &Bot, chat: &Chat) -> anyhow::Result<()> {
    let keyboard = vec![vec![KeyboardButton::new("Отмена")]];
    let keyboard_markup = KeyboardMarkup::new(keyboard).resize_keyboard(true);
    bot.send_message(chat.id, text::SEND_LIKE)
        .reply_markup(keyboard_markup)
        .await?;
    Ok(())
}

pub async fn handle_like_msg(
    db: Arc<Database>,
    dialogue: MyDialogue,
    bot: Bot,
    msg: Message,
    d: entities::datings::Model,
) -> anyhow::Result<()> {
    let Some(text) = msg.text() else {
        bot.send_message(msg.chat.id, "Лайк может содержать только текст! Для отмены отправьте \"Отмена\".").await?;
        bail!("message without text")
    };

    let msg_to_send = if text == "Отмена"
        || text.chars().next().context("empty string")? == '/'
    {
        db.set_dating_initiator_reaction(d.id, false).await?;
        "Отправка лайка отменена"
    } else {
        db.set_dating_initiator_reaction(d.id, true).await?;
        send_like(&db, &bot, &d, Some(text.to_owned())).await?;
        "Лайк отправлен!"
    };

    bot.send_message(msg.chat.id, msg_to_send)
        .reply_markup(KeyboardRemove::new())
        .await?;

    dialogue.exit().await?;
    send_recommendation(&bot, &db, ChatId(d.initiator_id)).await?;

    Ok(())
}
