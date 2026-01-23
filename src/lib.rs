use chrono::{DateTime, FixedOffset, Local};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub mod client;
pub mod config;
pub mod error;
pub mod server;

#[derive(Deserialize, Serialize, Default)]
pub struct Cookies {
    j_session_id: String,
    cookie: String,
    x_csrf_token: String,
}

impl Debug for Cookies {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cookies")
            .field(
                "j_session_id",
                &self
                    .j_session_id
                    .chars()
                    .take(5)
                    .chain("...".chars())
                    .collect::<String>(),
            )
            .field(
                "cookie",
                &self
                    .cookie
                    .chars()
                    .take(5)
                    .chain("...".chars())
                    .collect::<String>(),
            )
            .field(
                "x_csrf_token",
                &self
                    .x_csrf_token
                    .chars()
                    .take(5)
                    .chain("...".chars())
                    .collect::<String>(),
            )
            .finish()
    }
}

impl Cookies {
    fn cookie_sanitize(content: &str) -> String {
        content
            .chars()
            .filter(|&c| !matches!(c, ' ' | '"' | ',' | ';' | '\\') && !c.is_control())
            .collect()
    }

    pub fn sanitize(&self) -> Self {
        Cookies {
            cookie: Cookies::cookie_sanitize(&self.cookie),
            j_session_id: Cookies::cookie_sanitize(&self.j_session_id),
            x_csrf_token: self.x_csrf_token.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Records(Vec<(DateTime<Local>, f32)>);
