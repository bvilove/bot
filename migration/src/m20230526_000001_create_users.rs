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
            .create_type(
                Type::create()
                    .as_enum(LocationFilter::Table)
                    .values(LocationFilter::iter().skip(1))
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
                    .col(ColumnDef::new(Users::Name).string_len(16).not_null())
                    .col(
                        ColumnDef::new(Users::Gender)
                            .enumeration(Gender::Table, Gender::iter().skip(1))
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Users::GenderFilter)
                            .enumeration(Gender::Table, Gender::iter().skip(1)),
                    )
                    .col(
                        ColumnDef::new(Users::About)
                            .string_len(1024)
                            .not_null(),
                    )
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
                        ColumnDef::new(Users::GradeUpFilter)
                            .tiny_integer()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(Users::GradeDownFilter)
                            .tiny_integer()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(Users::Subjects)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Users::SubjectsFilter)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Users::DatingPurpose)
                            .small_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Users::City).integer().not_null())
                    .col(
                        ColumnDef::new(Users::LocationFilter)
                            .enumeration(
                                LocationFilter::Table,
                                LocationFilter::iter().skip(1),
                            )
                            .not_null(),
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
                            .integer()
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
                            .integer()
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
                    .col(ColumnDef::new(Datings::InitiatorMsgId).integer())
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
        manager
            .drop_type(Type::drop().name(LocationFilter::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(Iden, EnumIter)]
enum Gender {
    Table,
    Male,
    Female,
}

#[derive(Iden, EnumIter)]
enum LocationFilter {
    Table,
    SameCity,
    SameSubject,
    SameCounty,
    SameCountry,
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
    Name,
    Gender,
    GenderFilter,
    About,
    Active,
    LastActivity,
    GraduationYear,
    GradeUpFilter,
    GradeDownFilter,
    Subjects,
    SubjectsFilter,
    DatingPurpose,
    City,
    LocationFilter,
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
    InitiatorMsgId,
    Time,
    InitiatorReaction,
    PartnerReaction,
}
