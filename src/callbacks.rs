use std::{fmt::Display, str::FromStr};

use anyhow::{bail, Context};
use migration::Write;

use crate::types::{DatingPurpose, Subjects};

#[derive(PartialEq, Eq)]
pub enum RateCode {
    Dislike,
    LikeWithMsg,
    Like,
    ResponseDislike,
    ResponseLike,
}

impl From<&RateCode> for char {
    fn from(c: &RateCode) -> Self {
        match c {
            RateCode::Dislike => '👎',
            RateCode::LikeWithMsg => '💌',
            RateCode::Like => '👍',
            RateCode::ResponseDislike => '💔',
            RateCode::ResponseLike => '❤',
        }
    }
}

impl TryFrom<char> for RateCode {
    type Error = anyhow::Error;

    fn try_from(c: char) -> Result<Self, Self::Error> {
        Ok(match c {
            '👎' => Self::Dislike,
            '💌' => Self::LikeWithMsg,
            '👍' => Self::Like,
            '💔' => Self::ResponseDislike,
            '❤' => Self::ResponseLike,
            _ => bail!("can't parse RateCode"),
        })
    }
}

#[derive(PartialEq, Eq)]
pub enum UpdateBitflags<T> {
    Update(T),
    Continue,
}

impl<T> Display for UpdateBitflags<T>
where
    T: bitflags::Flags,
    <T as bitflags::Flags>::Bits: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Update(updated) => {
                f.write_fmt(format_args!("{}", updated.bits()))
            }
            Self::Continue => f.write_str("continue"),
        }
    }
}

#[derive(PartialEq, Eq)]
pub enum Callback {
    SetSubjects(UpdateBitflags<Subjects>),
    SetSubjectsFilter(UpdateBitflags<Subjects>),
    SetDatingPurpose(UpdateBitflags<DatingPurpose>),
    Edit,
    Dating { dating_id: i32, code: RateCode },
    CreateProfile,
    FindPartner,
}

impl Callback {
    pub fn char_id(&self) -> char {
        match self {
            Self::SetSubjects(_) => 's',
            Self::SetSubjectsFilter(_) => 'd',
            Self::SetDatingPurpose(_) => 'p',
            Self::Edit => 'e',
            Self::Dating { code, .. } => code.into(),
            Self::CreateProfile => '✍',
            Self::FindPartner => '🚀',
        }
    }
}

impl Display for Callback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char(self.char_id())?;
        match self {
            Self::SetSubjectsFilter(u) | Self::SetSubjects(u) => {
                f.write_fmt(format_args!("{u}"))?;
            }
            Self::SetDatingPurpose(u) => f.write_fmt(format_args!("{u}"))?,
            Self::Dating { dating_id, code: _ } => {
                f.write_fmt(format_args!("{dating_id}"))?;
            }
            Self::Edit | Self::CreateProfile | Self::FindPartner => {}
        };
        Ok(())
    }
}

impl FromStr for Callback {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.chars();
        let first_char = chars.next().context("can't get first char")?;
        let data: String = chars.collect();

        Ok(match first_char {
            's' | 'd' => {
                if data == "continue" {
                    match first_char {
                        's' => Self::SetSubjects(UpdateBitflags::Continue),
                        'd' => {
                            Self::SetSubjectsFilter(UpdateBitflags::Continue)
                        }
                        _ => bail!("this should never occur"), /* TODO: remove this */
                    }
                } else {
                    let subjects_int = data.parse()?;
                    let subjects = Subjects::from_bits(subjects_int)
                        .context("can't create subjects")?;

                    match first_char {
                        's' => {
                            Self::SetSubjects(UpdateBitflags::Update(subjects))
                        }
                        'd' => Self::SetSubjectsFilter(UpdateBitflags::Update(
                            subjects,
                        )),
                        _ => bail!("this should never occur"), /* TODO: remove this */
                    }
                }
            }
            'p' => {
                if data == "continue" {
                    Self::SetDatingPurpose(UpdateBitflags::Continue)
                } else {
                    let purpose_int = data.parse()?;
                    let purpose = DatingPurpose::from_bits(purpose_int)
                        .context("can't create purpose")?;
                    Self::SetDatingPurpose(UpdateBitflags::Update(purpose))
                }
            }
            'e' => Self::Edit,
            '✍' => Self::CreateProfile,
            '🚀' => Self::FindPartner,
            '👎' | '💌' | '👍' | '💔' | '❤' => {
                let dating_id = data.parse()?;
                let code = first_char.try_into()?;
                Self::Dating { dating_id, code }
            }
            _ => bail!("unknown code"),
        })
    }
}
