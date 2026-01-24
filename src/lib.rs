use crate::error::Result;
use chrono::{DateTime, Local};
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, path::Path, slice::Iter};
use tokio::{fs::File, io::AsyncRead};

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
pub struct Records(pub Vec<(DateTime<Local>, f32)>);

impl Records {
    pub fn sort(&mut self) {
        self.0.sort_by_key(|x| x.0);
    }

    /// 返回记录中最早和最晚的时间点.
    pub fn time_span(&self) -> Option<(DateTime<Local>, DateTime<Local>)> {
        if self.0.is_empty() {
            None
        } else {
            Some((
                self.0.iter().map(|x| x.0).min().unwrap(),
                self.0.iter().map(|x| x.0).max().unwrap(),
            ))
        }
    }

    pub fn iter(&self) -> Iter<'_, (DateTime<Local>, f32)> {
        self.0.iter()
    }

    pub async fn from_csv_file(csv_file: impl AsRef<Path>) -> Result<Self> {
        Self::from_csv(File::options().read(true).open(csv_file.as_ref()).await?).await
    }

    pub async fn from_csv<R: AsyncRead + Unpin + Send>(csv_content: R) -> Result<Self> {
        let rdr = csv_async::AsyncReaderBuilder::new()
            .has_headers(false)
            .create_reader(csv_content);
        let mut recs = rdr.into_records();
        let mut rsts = Vec::with_capacity(recs.size_hint().0);
        while let Some(rec) = recs.next().await {
            let rec = rec?;
            let rec: (DateTime<Local>, f32) = rec.deserialize(None)?;
            rsts.push(rec);
        }
        Ok(Self(rsts))
    }
}
