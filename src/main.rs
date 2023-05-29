use anyhow::Result;
use entities::sea_orm_active_enums::Gender;
use rand::Rng;
use std::sync::Arc;
use tokio::task::JoinSet;

mod db;

async fn run(database: Arc<db::Database>) -> Result<()> {
    for _ in 0..16384 {
        let id: i64 = rand::thread_rng().gen();
        database
            .create_user(
                id,
                &format!("Amogus {id}"),
                if id % 2 == 1 {
                    Gender::Female
                } else {
                    Gender::Male
                },
                Some(if id % 2 == 0 {
                    Gender::Female
                } else {
                    Gender::Male
                }),
                "Susmogus",
            )
            .await?;

        // database.create_subject_pref(i, entities::sea_orm_active_enums::Subject::Physics).await?;
        for j in 0..2 {
            database.create_image(id, vec![j; 1024]).await?;
        }

        for _ in 0..4 {
            match database.get_partner(id).await {
                Ok(p) => println!("Partner for {id}: {}", p.name),
                Err(_) => println!("error finding partner"),
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_test_writer()
        .init();

    let database = Arc::new(db::Database::new().await?);

    let mut tasks = JoinSet::<Result<()>>::new();

    for _ in 0..16 {
        tasks.spawn(run(database.clone()));
    }

    wait_tasks(tasks).await;

    Ok(())
}

async fn wait_tasks(mut tasks: JoinSet<Result<()>>) {
    while let Some(res) = tasks.join_next().await {
        if let Err(e) = res {
            panic!("{}", e);
        }
    }
}
