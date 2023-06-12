use std::{fmt::Display, str::FromStr};

use anyhow::bail;
use bitflags::bitflags;
use chrono::Datelike;
use entities::{sea_orm_active_enums::Gender, users};
use itertools::Itertools;

use crate::cities::City;

/// Gender of user
pub enum UserGender {
    Female,
    Male,
}

impl From<&Gender> for UserGender {
    fn from(value: &Gender) -> Self {
        match value {
            Gender::Female => Self::Female,
            Gender::Male => Self::Male,
        }
    }
}

impl From<UserGender> for Gender {
    fn from(value: UserGender) -> Self {
        match value {
            UserGender::Female => Self::Female,
            UserGender::Male => Self::Male,
        }
    }
}

impl FromStr for UserGender {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "Я парень" => Ok(Self::Male),
            "Я девушка" => Ok(Self::Female),
            _ => Err(()),
        }
    }
}

impl Display for UserGender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let emoji = match self {
            Self::Female => "♀️",
            Self::Male => "♂️",
        };

        f.write_str(emoji)
    }
}

/// Filter of partner's gender
pub enum GenderFilter {
    Female,
    Male,
    All,
}

impl From<&Option<Gender>> for GenderFilter {
    fn from(value: &Option<Gender>) -> Self {
        match value {
            Some(Gender::Female) => Self::Female,
            Some(Gender::Male) => Self::Male,
            None => Self::All,
        }
    }
}

impl From<GenderFilter> for Option<Gender> {
    fn from(value: GenderFilter) -> Self {
        match value {
            GenderFilter::Female => Some(Gender::Female),
            GenderFilter::Male => Some(Gender::Male),
            GenderFilter::All => None,
        }
    }
}

impl FromStr for GenderFilter {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "Девушку" => Ok(Self::Female),
            "Парня" => Ok(Self::Male),
            "Не важно" => Ok(Self::All),
            _ => Err(()),
        }
    }
}

impl Display for GenderFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let emoji = match self {
            Self::Female => "Девушку",
            Self::Male => "Парня",
            Self::All => "Не важно",
        };

        f.write_str(emoji)
    }
}

pub struct GraduationYear(i16);

impl From<i16> for GraduationYear {
    fn from(value: i16) -> Self {
        Self(value)
    }
}

impl From<GraduationYear> for i16 {
    fn from(value: GraduationYear) -> Self {
        value.0
    }
}

pub struct Grade(i8);

impl TryFrom<i8> for Grade {
    type Error = ();

    fn try_from(value: i8) -> Result<Self, Self::Error> {
        if (1..=11).contains(&value) {
            Ok(Self(value))
        } else {
            Err(())
        }
    }
}

impl Display for Grade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{} класс", self.0))
    }
}

impl From<Grade> for GraduationYear {
    fn from(grade: Grade) -> Self {
        let date = chrono::Local::now();

        let year = if date.month() < 9 {
            date.year() as i16 + (11 - grade.0 as i16)
        } else {
            date.year() as i16 + (11 - grade.0 as i16) + 1
        };

        Self(year)
    }
}

impl From<GraduationYear> for Grade {
    fn from(graduation_year: GraduationYear) -> Self {
        let date = chrono::Local::now();

        let grade = if date.month() < 9 {
            11 - (graduation_year.0 - date.year() as i16)
        } else {
            11 - (graduation_year.0 - date.year() as i16) + 1
        };

        Self(grade as i8)
    }
}

// pub struct UserSettings {
//     id: i64,
// }

// impl From<users::Model> for UserSettings {
//     fn from(value: users::Model) -> Self {
//         Self { id: value.id }
//     }
// }

/// Public profile of user
pub struct PublicProfile {
    name: String,
    gender: UserGender,
    grade: Grade,
    subjects: UserSubjects,
    dating_purpose: DatingPurpose,
    city: City,
    about: String,
}

impl TryFrom<&users::Model> for PublicProfile {
    type Error = anyhow::Error;

    fn try_from(value: &users::Model) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name.clone(),
            gender: (&value.gender).into(),
            grade: GraduationYear(value.graduation_year).into(),
            subjects: value.subjects.try_into()?,
            dating_purpose: value.dating_purpose.try_into()?,
            city: value.city.into(),
            about: value.about.clone(),
        })
    }
}

impl Display for PublicProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{} {}, {}.\n🔎 Интересует: {}.\n📚 {}\n.🧭 {}.\n\n{}",
            self.gender,
            self.name,
            self.grade,
            self.dating_purpose,
            self.subjects,
            self.city,
            self.about
        ))
    }
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

impl Subjects {
    /// Name of exactly one subject
    pub fn name(&self) -> std::result::Result<&'static str, ()> {
        Ok(match *self {
            Subjects::Art => "Искусство 🎨",
            Subjects::Astronomy => "Астрономия 🌌",
            Subjects::Biology => "Биология 🔬",
            Subjects::Chemistry => "Химия 🧪",
            Subjects::Chinese => "Китайский 🇨🇳",
            Subjects::Ecology => "Экология ♻️",
            Subjects::Economics => "Экономика 💶",
            Subjects::English => "Английский 🇬🇧",
            Subjects::French => "Французский 🇫🇷",
            Subjects::Geography => "География 🌎",
            Subjects::German => "Немецкий 🇩🇪",
            Subjects::History => "История 📰",
            Subjects::Informatics => "Информатика 💻",
            Subjects::Italian => "Итальянский 🇮🇹",
            Subjects::Law => "Право 👨‍⚖️",
            Subjects::Literature => "Литература 📖",
            Subjects::Math => "Математика 📐",
            Subjects::Physics => "Физика ☢️",
            Subjects::Russian => "Русский 🇷🇺",
            Subjects::Safety => "ОБЖ 🪖",
            Subjects::Social => "Обществознание 👫",
            Subjects::Spanish => "Испанский 🇪🇸",
            Subjects::Sport => "Физкультура 🏐",
            Subjects::Technology => "Технология 🚜",
            _ => return Err(()),
        })
    }
}

impl Display for Subjects {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, name) in Subjects::all()
            .into_iter()
            .filter(|s| self.contains(*s))
            .map(|s| s.name().unwrap())
            .sorted_unstable_by_key(|n| n.to_lowercase())
            .enumerate()
        {
            if i != 0 {
                f.write_str(", ")?;
            }
            f.write_str(name)?;
        }

        Ok(())
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

impl DatingPurpose {
    /// Name of exactly one purpose
    pub fn name(&self) -> std::result::Result<&'static str, ()> {
        Ok(match *self {
            DatingPurpose::Friendship => "Дружба 🧑‍🤝‍🧑",
            DatingPurpose::Studies => "Учёба 📚",
            DatingPurpose::Relationship => "Отношения 💕",
            _ => return Err(()),
        })
    }
}

impl Display for DatingPurpose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, name) in DatingPurpose::all()
            .into_iter()
            .filter(|p| self.contains(*p))
            .map(|p| p.name().unwrap())
            .enumerate()
        {
            if i != 0 {
                f.write_str(", ")?;
            }
            f.write_str(name)?;
        }

        Ok(())
    }
}

impl TryFrom<i16> for DatingPurpose {
    type Error = anyhow::Error;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        let Some(purpose) = DatingPurpose::from_bits(value) else {
            bail!("can't construct purpose from bits")
        };

        Ok(purpose)
    }
}

pub struct UserSubjects(Subjects);

impl Display for UserSubjects {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.bits() != 0 {
            f.write_fmt(format_args!("Ботает: {}", self.0))?;
        } else {
            f.write_str("Ничего не ботает.")?;
        }

        Ok(())
    }
}

impl TryFrom<i32> for UserSubjects {
    type Error = anyhow::Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        let Some(subjects) = Subjects::from_bits(value) else {
            bail!("can't construct subjects from bits")
        };

        Ok(Self(subjects))
    }
}
