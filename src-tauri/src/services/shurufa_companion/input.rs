use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InputId {
    EncoderCw,
    EncoderCcw,
    EncoderPress,
}

impl InputId {
    pub const ALL: [Self; 3] = [Self::EncoderCw, Self::EncoderCcw, Self::EncoderPress];
}

impl Display for InputId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let token = match self {
            Self::EncoderCw => "ENCODER_CW",
            Self::EncoderCcw => "ENCODER_CCW",
            Self::EncoderPress => "ENCODER_PRESS",
        };
        formatter.write_str(token)
    }
}

impl FromStr for InputId {
    type Err = InputError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "ENCODER_CW" => Ok(Self::EncoderCw),
            "ENCODER_CCW" => Ok(Self::EncoderCcw),
            "ENCODER_PRESS" => Ok(Self::EncoderPress),
            _ => Err(InputError::UnknownInput),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chord(pub Vec<String>);

impl Chord {
    pub fn parse(tokens: &[String]) -> Result<Self, InputError> {
        if tokens.is_empty() || tokens.len() > 4 {
            return Err(InputError::InvalidChord);
        }
        let normalized = tokens
            .iter()
            .map(|token| token.trim().to_ascii_uppercase())
            .collect::<Vec<_>>();
        if normalized.iter().any(String::is_empty) {
            return Err(InputError::InvalidChord);
        }
        let mut modifiers = Vec::new();
        for modifier in ["CTRL", "ALT", "SHIFT"] {
            if normalized.iter().any(|token| token == modifier) {
                modifiers.push(modifier.to_owned());
            }
        }
        if modifiers.len() >= normalized.len()
            || normalized
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
                != normalized.len()
        {
            return Err(InputError::InvalidChord);
        }
        let primaries = normalized
            .into_iter()
            .filter(|token| !["CTRL", "ALT", "SHIFT"].contains(&token.as_str()))
            .collect::<Vec<_>>();
        if primaries.is_empty() || primaries.iter().any(|token| !primary_is_allowed(token)) {
            return Err(InputError::InvalidChord);
        }
        modifiers.extend(primaries);
        Ok(Self(modifiers))
    }
    pub fn canonical(&self) -> String {
        self.0.join("+")
    }
}

fn primary_is_allowed(value: &str) -> bool {
    matches!(value, "ENTER" | "TAB" | "ESC" | "SPACE" | "[" | "]")
        || (value.len() == 1 && value.as_bytes()[0].is_ascii_uppercase())
        || (value.len() == 1 && value.as_bytes()[0].is_ascii_digit())
        || value
            .strip_prefix('F')
            .and_then(|number| number.parse::<u8>().ok())
            .is_some_and(|number| (1..=24).contains(&number))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputError {
    UnknownInput,
    InvalidChord,
}
impl Display for InputError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::UnknownInput => "unknown physical input",
            Self::InvalidChord => "invalid shortcut chord",
        })
    }
}
impl std::error::Error for InputError {}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chord_canonicalizes_order_and_rejects_duplicates() {
        let chord = Chord::parse(&["shift".into(), "ctrl".into(), "tab".into()]).unwrap();
        assert_eq!(chord.canonical(), "CTRL+SHIFT+TAB");
        let multi = Chord::parse(&["ctrl".into(), "tab".into(), "1".into()]).unwrap();
        assert_eq!(multi.canonical(), "CTRL+TAB+1");
        let bracket = Chord::parse(&["ctrl".into(), "shift".into(), "[".into()]).unwrap();
        assert_eq!(bracket.canonical(), "CTRL+SHIFT+[");
        assert!(Chord::parse(&["CTRL".into(), "CTRL".into(), "A".into()]).is_err());
        assert!(Chord::parse(&["CTRL".into(), "ALT".into()]).is_err());
    }
}
