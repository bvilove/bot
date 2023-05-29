use teloxide::{
    adaptors::{throttle::Limits, Throttle},
    dispatching::dialogue::InMemStorage,
    prelude::*,
    utils::command::BotCommands,
};

type Bot = Throttle<teloxide::Bot>;
type MyDialogue = Dialogue<State, InMemStorage<State>>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    tracing::info!("Starting bot...");
    let bot = teloxide::Bot::from_env()
        .throttle(Limits { messages_per_min_chat: 30, ..Default::default() });

    let handler = Update::filter_message()
        .enter_dialogue::<Message, InMemStorage<State>, State>()
        .branch(dptree::case![State::CreateAnketa(a)].endpoint(edit_anketa))
        .branch(dptree::case![State::EditName(a)].endpoint(edit_anketa))
        .branch(dptree::case![State::EditSubject(a)].endpoint(edit_anketa))
        .branch(dptree::case![State::EditDescription(a)].endpoint(edit_anketa))
        .branch(dptree::entry().filter_command::<Command>().endpoint(answer))
        .branch(dptree::endpoint(invalid_command));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![InMemStorage::<State>::new()])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
    Ok(())
}

struct Anketa {
    name: String,
    subject: String,
    description: String,
}

#[derive(Clone, Default)]
struct EditAnketa {
    name: Option<String>,
    subject: Option<String>,
    description: Option<String>,
}

#[derive(Clone, Default)]
enum State {
    #[default]
    Start,
    CreateAnketa(EditAnketa),
    EditName(EditAnketa),
    EditSubject(EditAnketa),
    EditDescription(EditAnketa),
}

#[derive(Debug, BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Доступные команды:")]
enum Command {
    #[command(description = "новая анкета (создать с нуля, э, но не совсем, \
                             типа если там 0 нажать, то прошлый вариант \
                             сохраниться, так что э кринж)")]
    CreateAnketa,
    #[command(description = "изменить анкету")]
    EditAnketa,
    #[command(description = "включить анкету")]
    EnableAnketa,
    #[command(description = "выключить анкета")]
    DisableAnketa,
    Help,
}

// #[tracing::instrument(skip(db, bot))]
async fn answer(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    cmd: Command,
) -> anyhow::Result<()> {
    match cmd {
        Command::CreateAnketa => {
            dialogue.update(State::CreateAnketa(EditAnketa::default())).await?;
            bot.send_message(msg.chat.id, EDIT_NAME_TEXT).await?;
        }
        Command::EditAnketa => {
            if get_anketa(msg.chat.id.0).await?.is_some() {
                dialogue.update(State::EditName(EditAnketa::default())).await?;
                bot.send_message(msg.chat.id, EDIT_NAME_TEXT).await?;
            } else {
                bot.send_message(msg.chat.id, "Сначала создайте анкету")
                    .await?;
            }
        }
        Command::EnableAnketa | Command::DisableAnketa => {
            todo!();
        }
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?;
        }
    }

    Ok(())
}

const EDIT_NAME_TEXT: &str = "Напиши имя от 3 до 20 символов (0 для пропуска).";

async fn edit_anketa(
    bot: Bot,
    dialogue: MyDialogue,
    mut anketa: EditAnketa,
    state: State,
    msg: Message,
) -> anyhow::Result<()> {
    const EDIT_SUBJECT_TEXT: &str = "Напиши предметы бота (0 для пропуска).";
    const EDIT_DESCRIPTION_TEXT: &str =
        "Напиши описание до 100 символов (0 для пропуска).";

    macro_rules! test {
        (
            $text:ident,
            $cur_text:expr,
            $next:expr,
            $validate:expr,
            $action:expr,
            $next_text:expr
        ) => {
            match msg.text() {
                Some("0") => {
                    bot.send_message(msg.chat.id, $next_text).await?;
                    dialogue.update($next).await?;
                }
                Some($text) if $validate => {
                    $action;
                    bot.send_message(msg.chat.id, $next_text).await?;
                    dialogue.update($next).await?;
                }
                _ => {
                    bot.send_message(msg.chat.id, $cur_text).await?;
                }
            }
        };
        (
            disable_skip
            $text:ident,
            $cur_text:expr,
            $next:expr,
            $validate:expr,
            $action:expr,
            $next_text:expr
        ) => {
            match msg.text() {
                Some($text) if $validate => {
                    $action;
                    bot.send_message(msg.chat.id, $next_text).await?;
                    dialogue.update($next).await?;
                }
                _ => {
                    bot.send_message(msg.chat.id, $cur_text).await?;
                }
            }
        };
    }

    match state {
        State::CreateAnketa(_) => test!(
            disable_skip
            text,
            EDIT_NAME_TEXT,
            State::EditSubject(anketa),
            (3..=30).contains(&text.len()),
            anketa.name = Some(text.to_owned()),
            EDIT_SUBJECT_TEXT
        ),
        State::EditName(_) => test!(
            text,
            EDIT_NAME_TEXT,
            State::EditSubject(anketa),
            (3..=30).contains(&text.len()),
            anketa.name = Some(text.to_owned()),
            EDIT_SUBJECT_TEXT
        ),
        State::EditSubject(_) => test!(
            text,
            EDIT_SUBJECT_TEXT,
            State::EditDescription(anketa),
            (1..=100).contains(&text.len()),
            anketa.subject = Some(text.to_owned()),
            EDIT_DESCRIPTION_TEXT
        ),
        State::EditDescription(_) => test!(
            text,
            EDIT_DESCRIPTION_TEXT,
            State::Start,
            (1..=100).contains(&text.len()),
            {
                anketa.description = Some(text.to_owned());
                save_anketa_to_db(&anketa).await?;
            },
            format!(
                "Ваша анкета:\nИмя:{:?}\nПредмет:{:?}\nОписание:{:?}",
                anketa.name, anketa.subject, anketa.description
            )
        ),
        _ => {}
    }
    Ok(())
}

async fn invalid_command(bot: Bot, msg: Message) -> anyhow::Result<()> {
    bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?;
    Ok(())
}

async fn save_anketa_to_db(_anketa: &EditAnketa) -> anyhow::Result<()> {
    Ok(())
}

async fn get_anketa(_id: i64) -> anyhow::Result<Option<Anketa>> {
    Ok(Some(Anketa {
        name: "br".to_owned(),
        subject: "what".to_owned(),
        description: "j".to_owned(),
    }))
}
