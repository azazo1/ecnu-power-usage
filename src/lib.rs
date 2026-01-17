use serde::{Deserialize, Serialize};

pub mod config;
pub mod error;
pub mod server;

#[derive(Deserialize, Serialize, Default)]
pub struct Cookies {
    #[serde(rename = "JSESSIONID")]
    j_session_id: String,
    #[serde(rename = "cookie")]
    cookie: String,
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
        }
    }
}
