use std::{
    convert::Infallible,
    fmt::{Debug, Display},
};

use reqwest::StatusCode;
use url::ParseError;

pub enum GrmError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Reqwest(reqwest::Error),
    UrlParse(ParseError),
    Custom(String),
    Infallible(Infallible),
}

macro_rules! impl_from {
    ($from:ty => $variant:ident) => {
        impl From<$from> for GrmError {
            fn from(value: $from) -> Self {
                GrmError::$variant(value)
            }
        }
    };
}

impl_from!(std::io::Error => Io);
impl_from!(serde_json::Error => Json);
impl_from!(reqwest::Error => Reqwest);
impl_from!(url::ParseError => UrlParse);
impl_from!(String => Custom);
impl_from!(Infallible => Infallible);

impl Display for GrmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GrmError::Io(e) => write!(f, "Filesystem error: {}", e),
            GrmError::Json(e) => write!(f, "Configuration format error: {}", e),
            GrmError::Reqwest(e) => {
                if let Some(status) = e.status() {
                    match status {
                        StatusCode::FORBIDDEN => {
                            write!(f, "GitHub API access forbidden. You might be rate-limited.")
                        }
                        StatusCode::NOT_FOUND => write!(
                            f,
                            "Requested resource not found. Check if the repo/owner exists."
                        ),
                        s => write!(f, "GitHub API returned error status: {}", s),
                    }
                } else {
                    write!(f, "Network connection error: {}", e)
                }
            }
            GrmError::UrlParse(e) => write!(f, "Invalid URL generated: {}", e),
            GrmError::Custom(e) => write!(f, "{}", e),
            GrmError::Infallible(_) => write!(
                f,
                "An operation guranteed to succeed has failed, please report this."
            ),
        }
    }
}

impl Debug for GrmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}
