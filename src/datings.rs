use std::sync::Arc;

use anyhow::Context;
use teloxide::{
    prelude::*,
    types::{Chat, InputFile, InputMedia, InputMediaPhoto},
};

use crate::{db::Database, Bot};

pub async fn send_recommendation(
    bot: Bot,
    chat: Chat,
    db: Arc<Database>,
) -> anyhow::Result<()> {
    let partner = db.get_partner(chat.id.0).await?;
    let partner_images = db.get_images(partner.id).await?;

    let subjects = if partner.subjects != 0 {
        crate::utils::subjects_list(
            crate::Subjects::from_bits(partner.subjects)
                .context("subjects must be created")?,
        )?
    } else {
        "Ничего не ботает.".to_owned()
    };

    let grade = crate::utils::grade_from_graduation_year(
        partner.graduation_year.into(),
    )?;

    let city = crate::cities::format_city(partner.city)?;

    let partner_msg = format!(
        "{}, {} класс.\n{}.\n\n🧭 {}.\n\n{}",
        partner.name, grade, subjects, city, partner.about
    );

    if partner_images.is_empty() {
        bot.send_message(chat.id, partner_msg).await?;
    } else {
        let medias =
            partner_images.into_iter().enumerate().map(|(index, id)| {
                let input_file = InputFile::file_id(id);
                let mut input_media_photo = InputMediaPhoto::new(input_file);
                if index == 0 {
                    input_media_photo = input_media_photo.caption(&partner_msg)
                }
                InputMedia::Photo(input_media_photo)
            });
        bot.send_media_group(chat.id, medias).await?;
    }

    Ok(())
}
