//! `SeaORM` Entity. Generated by sea-orm-codegen 0.11.3

use sea_orm::entity::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "gender")]
pub enum Gender {
    #[sea_orm(string_value = "female")]
    Female,
    #[sea_orm(string_value = "male")]
    Male,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "image_kind")]
pub enum ImageKind {
    #[sea_orm(string_value = "image")]
    Image,
    #[sea_orm(string_value = "video")]
    Video,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "location_filter")]
pub enum LocationFilter {
    #[sea_orm(string_value = "same_city")]
    SameCity,
    #[sea_orm(string_value = "same_country")]
    SameCountry,
    #[sea_orm(string_value = "same_county")]
    SameCounty,
    #[sea_orm(string_value = "same_subject")]
    SameSubject,
}
