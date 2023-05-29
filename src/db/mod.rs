use anyhow::Context;
use anyhow::Result;
use chrono::prelude::*;
use chrono::Duration;
use entities::prelude::*;
use entities::sea_orm_active_enums::*;
use entities::*;
// use migration::Alias;
// use migration::BinOper;
// use migration::Expr;
// use migration::SimpleExpr;
use migration::{Migrator, MigratorTrait};
use sea_orm::*;
use sea_orm::{Database as SeaDatabase, DatabaseConnection};
use sea_query::*;

pub struct Database {
    conn: DatabaseConnection,
}

impl Database {
    pub async fn new() -> Result<Self> {
        let db_url = std::env::var("DATABASE_URL")?;
        let conn = SeaDatabase::connect(db_url).await?;
        Migrator::up(&conn, None).await?;
        Ok(Self { conn })
    }

    pub async fn create_user(
        &self,
        id: i64,
        name: &str,
        gender: Gender,
        gender_pref: Option<Gender>,
        about: &str,
    ) -> Result<()> {
        let now_utc = Utc::now();
        let now_naive = NaiveDateTime::from_timestamp_micros(now_utc.timestamp_micros())
            .expect("naive time must be created");

        let user = users::ActiveModel {
            id: ActiveValue::Set(id),
            name: ActiveValue::Set(name.to_owned()),
            gender: ActiveValue::Set(gender),
            gender_pref: ActiveValue::Set(gender_pref),
            about: ActiveValue::Set(about.to_owned()),
            active: ActiveValue::Set(true),
            last_activity: ActiveValue::Set(now_naive),
            graduation_year: ActiveValue::Set(2023),
            up_graduation_year_delta_pref: ActiveValue::Set(1),
            down_graduation_year_delta_pref: ActiveValue::Set(1),
            subjects: ActiveValue::Set(3),
            subjects_prefs: ActiveValue::Set(1),
        };
        Users::insert(user).exec(&self.conn).await?;
        Ok(())
    }

    pub async fn create_image(&self, user_id: i64, data: Vec<u8>) -> Result<()> {
        let image = entities::images::ActiveModel {
            id: NotSet,
            data: ActiveValue::Set(data),
            user_id: ActiveValue::Set(user_id),
        };
        Images::insert(image).exec(&self.conn).await?;
        Ok(())
    }

    pub async fn _get_user_info(&self, id: i64) -> Result<users::Model> {
        Users::find_by_id(id)
            .one(&self.conn)
            .await?
            .context("user not found")
    }

    pub async fn get_partner(&self, user_id: i64) -> Result<users::Model> {
        let now_utc = Utc::now();
        let now_naive = NaiveDateTime::from_timestamp_micros(now_utc.timestamp_micros())
            .expect("naive time must be created");
        let week_ago_utc = now_utc - Duration::weeks(1);
        let week_ago_naive = NaiveDateTime::from_timestamp_micros(week_ago_utc.timestamp_micros())
            .expect("naive time must be created");

        // Load dating initiator
        let user = Users::find_by_id(user_id)
            .one(&self.conn)
            .await?
            .context("user not found")?;

        // let subject_prefs = user
        //     .find_related(subject_preferences::Entity)
        //     .all(&self.conn)
        //     .await?;

        // println!("{:?}", subject_prefs);

        let user_id_clone = user.id.clone();

        let mut partner_query = Users::find()
            // Don't recommend user to himself
            .filter(users::Column::Id.ne(user_id))
            // Only recommend activated profiles
            .filter(users::Column::Active.eq(true))
            // Only recommend active users
            .filter(users::Column::LastActivity.gt(week_ago_naive))
            // Respect users's graduation delta preference
            .filter(users::Column::GraduationYear.between(
                user.graduation_year - user.down_graduation_year_delta_pref,
                user.graduation_year + user.up_graduation_year_delta_pref,
            ))
            // Respect partner's graduation delta preference
            .filter(
                users::Column::GraduationYear
                    .into_expr()
                    .add(users::Column::UpGraduationYearDeltaPref.into_expr())
                    .binary(
                        BinOper::GreaterThanOrEqual,
                        Expr::value(user.graduation_year),
                    )
                    .and(
                        users::Column::GraduationYear
                            .into_expr()
                            .sub(users::Column::DownGraduationYearDeltaPref.into_expr())
                            .binary(
                                BinOper::SmallerThanOrEqual,
                                Expr::value(user.graduation_year),
                            ),
                    ),
            )
            // Respect partner's subject preference
            .filter(
                Condition::any()
                    .add(
                        Expr::cust_with_exprs(
                            "$1 & $2",
                            [
                                users::Column::SubjectsPrefs
                                    .into_expr()
                                    .cast_as(Alias::new("bit(64)")),
                                Expr::value(user.subjects).cast_as(Alias::new("bit(64)")),
                            ],
                        )
                        .ne(Expr::value(0i64).cast_as(Alias::new("bit(64)"))),
                    )
                    .add(users::Column::SubjectsPrefs.eq(0i64)),
            )
            // Respect partner's gender preference
            .filter(
                Condition::any()
                    .add(users::Column::GenderPref.eq(None as Option<Gender>))
                    .add(users::Column::GenderPref.eq(Some(user.gender.clone()))),
            )
            // Don't recommend the same partner more than once a week
            .join_rev(
                JoinType::LeftJoin,
                datings::Entity::belongs_to(users::Entity)
                    .from(datings::Column::PartnerId)
                    .to(users::Column::Id)
                    .on_condition(move |_left, _right| {
                        datings::Column::InitiatorId
                            .eq(user_id_clone)
                            .and(datings::Column::Time.gt(week_ago_naive))
                            .into_condition()
                    })
                    .into(),
            )
            .having(datings::Column::Id.count().eq(0))
            .group_by(users::Column::Id)
            // Get random partner
            .order_by_desc(sea_query::SimpleExpr::from(sea_query::Func::random()));

        // Respect user's subject preference
        if user.subjects_prefs != 0 {
            partner_query = partner_query.filter(
                Expr::cust_with_exprs(
                    "$1 & $2",
                    [
                        users::Column::Subjects
                            .into_expr()
                            .cast_as(Alias::new("bit(64)")),
                        Expr::value(user.subjects_prefs).cast_as(Alias::new("bit(64)")),
                    ],
                )
                .ne(Expr::value(0i64).cast_as(Alias::new("bit(64)"))),
            );
        }

        // Respect user's gender preference
        if let Some(g) = &user.gender_pref {
            partner_query = partner_query.filter(users::Column::Gender.eq(Some(g.clone())));
        }

        let partner = partner_query
            .one(&self.conn)
            .await?
            .context("partner not found")?;

        // Save dating
        let dating = datings::ActiveModel {
            id: NotSet,
            initiator_id: ActiveValue::Set(user_id),
            partner_id: ActiveValue::Set(partner.id),
            time: ActiveValue::Set(now_naive),
            initiator_reaction: NotSet,
            partner_reaction: NotSet,
        };
        Datings::insert(dating).exec(&self.conn).await?;

        // Update last activity datetime
        let mut active_user: users::ActiveModel = user.into();
        active_user.last_activity = Set(now_naive);
        active_user.update(&self.conn).await?;

        Ok(partner)
    }
}
