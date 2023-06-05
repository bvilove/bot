#![feature(lazy_cell)]

use std::sync::Arc;

use bitflags::bitflags;
use db::Database;
use entities::sea_orm_active_enums::Gender;
use teloxide::{
    adaptors::{throttle::Limits, Throttle},
    dispatching::dialogue::InMemStorage,
    prelude::*,
    utils::command::BotCommands,
};

mod cities;
mod datings;
mod db;
mod handle;
mod request;
mod text;
mod utils;

type Bot = Throttle<teloxide::Bot>;
type MyDialogue = Dialogue<State, InMemStorage<State>>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use handle::*;

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
                    dptree::case![State::SetCity(a)].endpoint(handle_set_city),
                )
                .branch(
                    dptree::case![State::SetPartnerCity(a)]
                        .endpoint(handle_set_partner_city),
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
    pub struct Subjects: i64 {
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

#[derive(Clone, Default, Debug)]
pub struct NewProfile {
    name: Option<String>,
    gender: Option<Gender>,
    graduation_year: Option<i16>,
    subjects: Option<Subjects>,
    partner_subjects: Option<Subjects>,
    about: Option<String>,
    partner_gender: Option<Gender>,
    city: Option<i16>,
    same_partner_city: Option<bool>,
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
    city: i16,
    same_partner_city: bool,
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
                partner_gender,
                city: Some(city),
                same_partner_city: Some(same_partner_city),
            } => Ok(Profile {
                name,
                gender,
                graduation_year: grade,
                subjects,
                partner_subjects,
                about,
                partner_gender,
                city,
                same_partner_city,
            }),
            _ => {
                anyhow::bail!("can't create Profile from NewProfile: {:?}", new)
            }
        }
    }
}

#[derive(Clone, Default, Debug)]
pub enum State {
    #[default]
    Start,
    SetName(NewProfile),
    SetGender(NewProfile),
    SetPartnerGender(NewProfile),
    SetGraduationYear(NewProfile),
    SetSubjects(NewProfile),
    SetPartnerSubjects(NewProfile),
    SetCity(NewProfile),
    SetPartnerCity(NewProfile),
    SetAbout(NewProfile),
}

#[derive(Debug, BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Доступные команды:")]
enum Command {
    #[command(description = "новая анкета")]
    NewProfile,
    #[command(description = "изменить анкету")]
    EditProfile,
    Recommend,
    // #[command(description = "включить анкету")]
    // EnableAnketa,
    // #[command(description = "выключить анкета")]
    // DisableAnketa,
    Help,
}

// #[tracing::instrument(skip(db, bot))]
async fn answer(
    db: Arc<Database>,
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    cmd: Command,
) -> anyhow::Result<()> {
    match cmd {
        Command::NewProfile => {
            dialogue.update(State::SetName(NewProfile::default())).await?;
            request::request_set_name(bot, msg.chat).await?;
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
        Command::Recommend => {
            datings::send_recommendation(bot, msg.chat, db).await?;
        }
    }

    Ok(())
}

async fn invalid_command(bot: Bot, msg: Message) -> anyhow::Result<()> {
    bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?;
    Ok(())
}
