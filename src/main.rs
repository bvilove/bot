use std::sync::Arc;

use bitflags::bitflags;
use db::Database;
use entities::sea_orm_active_enums::{Gender, LocationFilter};
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
    let bot = teloxide::Bot::from_env().throttle(Limits {
        messages_per_sec_chat: 2,
        messages_per_min_chat: 120,
        ..Default::default()
    });

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
                    dptree::case![State::SetGenderFilter(a)]
                        .endpoint(handle_set_partner_gender),
                )
                .branch(
                    dptree::case![State::SetGraduationYear(a)]
                        .endpoint(handle_set_grade),
                )
                .branch(
                    dptree::case![State::SetCity(a)].endpoint(handle_set_city),
                )
                .branch(
                    dptree::case![State::SetLocationFilter(a)]
                        .endpoint(handle_set_partner_city),
                )
                .branch(
                    dptree::case![State::SetAbout(a)]
                        .endpoint(handle_set_about),
                )
                .branch(
                    dptree::case![State::SetPhotos(a)]
                        .endpoint(handle_set_photos),
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
                    dptree::case![State::SetSubjectsFilter(a)]
                        .endpoint(handle_set_subjects_filter_callback),
                )
                .branch(
                    dptree::case![State::SetDatingPurpose(a)]
                        .endpoint(handle_set_dating_purpose_callback),
                )
                .branch(dptree::entry())
                .endpoint(datings::handle_dating_callback),
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
    pub struct Subjects: i32 {
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

bitflags! {
    #[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
    pub struct DatingPurpose: i16 {
        const Friendship = 1 << 0;
        const Studies = 1 << 1;
        const Relationship = 1 << 2;
    }
}

macro_rules! make_profile {
    ($($element:ident: $ty:ty),* $(,)?) => {
        #[derive(Clone, Default, Debug)]
        pub struct EditProfile {
            id: i64,
            create_new: bool,
            photos_count: u8,
            $($element: Option<$ty>),*
        }
        impl EditProfile {
            pub fn new(id: i64) -> Self {
                Self {
                    id,
                    create_new: true,
                    photos_count: 0,
                    name: None,
                    gender: None,
                    gender_filter: None,
                    about: None,
                    active: None,
                    graduation_year: None,
                    grade_up_filter: None,
                    grade_down_filter: None,
                    subjects: None,
                    subjects_filter: None,
                    dating_purpose: None,
                    city: None,
                    location_filter: None
                }
            }
        }
    };
}

make_profile!(
    name: String,
    gender: Gender,
    gender_filter: Option<Gender>,
    about: String,
    active: bool,
    graduation_year: i16,
    grade_up_filter: i16,
    grade_down_filter: i16,
    subjects: i32,
    subjects_filter: i32,
    dating_purpose: i16,
    city: i32,
    location_filter: LocationFilter,
);

#[derive(Clone, Default, Debug)]
pub enum State {
    #[default]
    Start,
    SetName(EditProfile),
    SetGender(EditProfile),
    SetGenderFilter(EditProfile),
    SetGraduationYear(EditProfile),
    SetSubjects(EditProfile),
    SetSubjectsFilter(EditProfile),
    SetDatingPurpose(EditProfile),
    SetCity(EditProfile),
    SetLocationFilter(EditProfile),
    SetAbout(EditProfile),
    SetPhotos(EditProfile),
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
            let profile = EditProfile::new(msg.chat.id.0);
            let state = State::SetName(profile.clone());
            handle::print_current_state(&state, profile, bot, msg.chat).await?;
            dialogue.update(state).await?;
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
            datings::send_recommendation(&bot, &db, msg.chat.id).await?;
        }
    }

    Ok(())
}

async fn invalid_command(bot: Bot, msg: Message) -> anyhow::Result<()> {
    bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?;
    Ok(())
}
