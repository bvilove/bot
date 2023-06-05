use std::sync::Arc;

use anyhow::Context;
use teloxide::{prelude::*, types::Chat};

use crate::{db::Database, Bot};

pub async fn send_recommendation(
    bot: Bot,
    chat: Chat,
    db: Arc<Database>,
) -> anyhow::Result<()> {
    let partner = db.get_partner(chat.id.0).await?;

    let subjects = if partner.subjects != 0 {
        format!(
            "ботает: {}",
            crate::utils::subjects_list(
                crate::Subjects::from_bits(partner.subjects)
                    .context("subjects must be created")?
            )?
        )
    } else {
        "ничего не ботает".to_owned()
    };

    let grade =
        crate::utils::grade_from_graduation_year(partner.graduation_year.into())?;

    let partner_msg = format!(
        "{}, {} класс, {}.\n{}",
        partner.name, grade, subjects, partner.about
    );

    bot.send_message(chat.id, partner_msg).await?;

    Ok(())
}
