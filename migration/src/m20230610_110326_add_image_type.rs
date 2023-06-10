use sea_orm::{sea_query::extension::postgres::Type, EnumIter, Iterable};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_type(
                Type::create()
                    .as_enum(ImageKind::Table)
                    .values(ImageKind::iter().skip(1))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Images::Table)
                    .add_column(
                        ColumnDef::new(Images::Kind)
                            .enumeration(
                                ImageKind::Table,
                                ImageKind::iter().skip(1),
                            )
                            .not_null()
                            .default(SimpleExpr::Custom(
                                "CAST('image' AS image_kind)".to_string(),
                            )),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(ImageKind::Table)
                    .drop_column(Images::Kind)
                    .to_owned(),
            )
            .await?;

        manager.drop_type(Type::drop().name(ImageKind::Table).to_owned()).await
    }
}

#[derive(Iden, EnumIter)]
enum ImageKind {
    Table,
    Image,
    Video,
}

#[derive(Iden)]
enum Images {
    Table,
    Kind,
}
