use sea_orm::{EnumIter, Iterable};
use sea_orm_migration::{prelude::*, sea_query::extension::postgres::Type};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_type(
                Type::create()
                    .as_enum(Gender::Table)
                    .values(Gender::iter().skip(1))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .col(
                        ColumnDef::new(Users::Id)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Users::Name).string().not_null())
                    .col(
                        ColumnDef::new(Users::Gender)
                            .enumeration(Gender::Table, Gender::iter().skip(1))
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Users::GenderPref)
                            .enumeration(Gender::Table, Gender::iter().skip(1)),
                    )
                    .col(ColumnDef::new(Users::About).string().not_null())
                    .col(
                        ColumnDef::new(Users::Active)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(Users::LastActivity)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Users::GraduationYear)
                            .small_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Users::UpGraduationYearDeltaPref)
                            .tiny_integer()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(Users::DownGraduationYearDeltaPref)
                            .tiny_integer()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(Users::Subjects)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Users::SubjectsPrefs)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(Users::City).integer().not_null())
                    .col(
                        ColumnDef::new(Users::SameCityPref)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Images::Table)
                    .col(
                        ColumnDef::new(Images::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Images::UserId).big_integer().not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Images::Table, Images::UserId)
                            .to(Users::Table, Users::Id),
                    )
                    .col(ColumnDef::new(Images::TelegramId).string().not_null())
                    .col(
                        ColumnDef::new(Images::Data)
                            .blob(BlobSize::Long)
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Datings::Table)
                    .col(
                        ColumnDef::new(Datings::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Datings::InitiatorId)
                            .big_integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Datings::Table, Datings::InitiatorId)
                            .to(Users::Table, Users::Id),
                    )
                    .col(
                        ColumnDef::new(Datings::PartnerId)
                            .big_integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Datings::Table, Datings::PartnerId)
                            .to(Users::Table, Users::Id),
                    )
                    .col(
                        ColumnDef::new(Datings::Time)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(Datings::InitiatorReaction).boolean())
                    .col(ColumnDef::new(Datings::PartnerReaction).boolean())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(Datings::Table)
                    .col(Datings::InitiatorId)
                    .col(Datings::PartnerId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Images::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await?;
        manager.drop_type(Type::drop().name(Gender::Table).to_owned()).await?;

        Ok(())
    }
}

#[derive(Iden, EnumIter)]
enum Gender {
    Table,
    Male,
    Female,
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
    Name,
    Gender,
    GenderPref,
    About,
    Active,
    LastActivity,
    GraduationYear,
    UpGraduationYearDeltaPref,
    DownGraduationYearDeltaPref,
    Subjects,
    SubjectsPrefs,
    City,
    SameCityPref,
}

#[derive(Iden)]
enum Images {
    Table,
    Id,
    UserId,
    TelegramId,
    Data,
}

#[derive(Iden)]
enum Datings {
    Table,
    Id,
    InitiatorId,
    PartnerId,
    Time,
    InitiatorReaction,
    PartnerReaction,
}
