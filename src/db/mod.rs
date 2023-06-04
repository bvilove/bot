use anyhow::{Context, Result};
use chrono::{prelude::*, Duration};
use entities::{prelude::*, sea_orm_active_enums::*, *};
use migration::{Migrator, MigratorTrait};
use sea_orm::{Database as SeaDatabase, DatabaseConnection, *};
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
        name: String,
        about: String,
        gender: Gender,
        gender_pref: Option<Gender>,
        graduation_year: i16,
        subjects: i64,
        subjects_prefs: i64,
        city: i16,
        same_city_pref: bool,
    ) -> Result<()> {
        let user = users::ActiveModel {
            id: ActiveValue::Set(id),
            name: ActiveValue::Set(name),
            about: ActiveValue::Set(about),
            gender: ActiveValue::Set(gender),
            gender_pref: ActiveValue::Set(gender_pref),
            graduation_year: ActiveValue::Set(graduation_year),
            subjects: ActiveValue::Set(subjects),
            subjects_prefs: ActiveValue::Set(subjects_prefs),
            city: ActiveValue::Set(city),
            same_city_pref: ActiveValue::Set(same_city_pref),
            ..Default::default()
        };
        Users::insert(user).exec(&self.conn).await?;
        Ok(())
    }

    pub async fn _create_image(
        &self,
        user_id: i64,
        data: Vec<u8>,
    ) -> Result<()> {
        let image = entities::images::ActiveModel {
            data: ActiveValue::Set(data),
            user_id: ActiveValue::Set(user_id),
            ..Default::default()
        };
        Images::insert(image).exec(&self.conn).await?;
        Ok(())
    }

    pub async fn _get_user_info(&self, id: i64) -> Result<users::Model> {
        Users::find_by_id(id).one(&self.conn).await?.context("user not found")
    }

    pub async fn _get_partner(&self, user_id: i64) -> Result<users::Model> {
        let now_utc = Utc::now();
        let now_naive =
            NaiveDateTime::from_timestamp_micros(now_utc.timestamp_micros())
                .expect("naive time must be created");
        let week_ago_utc = now_utc - Duration::weeks(1);
        let week_ago_naive = NaiveDateTime::from_timestamp_micros(
            week_ago_utc.timestamp_micros(),
        )
        .expect("naive time must be created");

        // Load dating initiator
        let user = Users::find_by_id(user_id)
            .one(&self.conn)
            .await?
            .context("user not found")?;

        // Update last activity datetime
        let mut active_user: users::ActiveModel = user.clone().into();
        active_user.last_activity = Set(now_naive);
        active_user.update(&self.conn).await?;

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
                            .sub(
                                users::Column::DownGraduationYearDeltaPref
                                    .into_expr(),
                            )
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
                        Expr::cust_with_exprs("$1 & $2", [
                            users::Column::SubjectsPrefs
                                .into_expr()
                                .cast_as(Alias::new("bit(64)")),
                            Expr::value(user.subjects)
                                .cast_as(Alias::new("bit(64)")),
                        ])
                        .ne(Expr::value(0i64).cast_as(Alias::new("bit(64)"))),
                    )
                    .add(users::Column::SubjectsPrefs.eq(0i64)),
            )
            // Respect partner's gender preference
            .filter(
                Condition::any()
                    .add(users::Column::GenderPref.eq(None as Option<Gender>))
                    .add(
                        users::Column::GenderPref.eq(Some(user.gender.clone())),
                    ),
            )
            // Respect partner's city preference
            .filter(
                Condition::any()
                    .add(users::Column::SameCityPref.eq(false))
                    .add(users::Column::City.eq(user.city)),
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
            .group_by(users::Column::Id)
            .having(datings::Column::Id.count().eq(0))
            // Get random partner
            .order_by_desc(SimpleExpr::from(Func::random()));

        // Respect user's subject preference
        if user.subjects_prefs != 0 {
            partner_query = partner_query.filter(
                Expr::cust_with_exprs("$1 & $2", [
                    users::Column::Subjects
                        .into_expr()
                        .cast_as(Alias::new("bit(64)")),
                    Expr::value(user.subjects_prefs)
                        .cast_as(Alias::new("bit(64)")),
                ])
                .ne(Expr::value(0i64).cast_as(Alias::new("bit(64)"))),
            );
        }

        // Respect user's gender preference
        if let Some(g) = &user.gender_pref {
            partner_query =
                partner_query.filter(users::Column::Gender.eq(Some(g.clone())));
        }

        // Respect user's city preference
        if user.same_city_pref {
            partner_query =
                partner_query.filter(users::Column::City.eq(user.city));
        }

        let txn = self.conn.begin().await?;

        let partner =
            partner_query.one(&txn).await?.context("partner not found")?;

        // Save dating
        let dating = datings::ActiveModel {
            initiator_id: ActiveValue::Set(user_id),
            partner_id: ActiveValue::Set(partner.id),
            ..Default::default()
        };
        Datings::insert(dating).exec(&txn).await?;

        txn.commit().await?;

        Ok(partner)
    }
}
