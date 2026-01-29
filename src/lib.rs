use chrono::{DateTime, FixedOffset};
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, path::Path, slice::Iter};
use tokio::{fs::File, io::AsyncRead};

pub mod client;
pub mod config;
pub mod error;
pub mod server;

pub use error::{CSError, Error, Result};
pub use server::{ArchiveMeta, TimeSpan};

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
    pub fn empty() -> Self {
        Self {
            ..Default::default()
        }
    }

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
pub struct Records(pub Vec<(DateTime<FixedOffset>, f32)>);

impl Records {
    pub fn sort(&mut self) {
        self.0.sort_by_key(|x| x.0);
    }

    /// 返回记录中最早和最晚的时间点.
    pub fn time_span(&self) -> Option<(DateTime<FixedOffset>, DateTime<FixedOffset>)> {
        if self.0.is_empty() {
            None
        } else {
            Some((
                self.0.iter().map(|x| x.0).min().unwrap(),
                self.0.iter().map(|x| x.0).max().unwrap(),
            ))
        }
    }

    pub fn iter(&self) -> Iter<'_, (DateTime<FixedOffset>, f32)> {
        self.0.iter()
    }

    pub async fn to_csv(&self) -> Result<String> {
        let mut ser = csv_async::AsyncWriterBuilder::new().create_serializer(vec![]);
        for rec in &self.0 {
            ser.serialize(rec).await?;
        }
        // unwrap: 在内存中写入不会报错.
        Ok(String::from_utf8(ser.into_inner().await.unwrap())?)
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
            let rec: (DateTime<FixedOffset>, f32) = rec.deserialize(None)?;
            rsts.push(rec);
        }
        Ok(Self(rsts))
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use chrono::DateTime;

    use crate::Records;

    #[tokio::test]
    async fn load_records() {
        let recs = Records::from_csv(Cursor::new(
            "\
2026-01-24T14:35:32+08:00,33.43
2026-01-25T00:00:00+08:00,10.00
",
        ))
        .await
        .unwrap();

        assert_eq!(
            recs.0,
            vec![
                (
                    DateTime::from_timestamp_secs(1769236532)
                        .unwrap()
                        .fixed_offset(),
                    33.43f32
                ),
                (
                    // python3.14: int(datetime.datetime(year=2026, month=1, day=25).timestamp())
                    DateTime::from_timestamp_secs(1769270400)
                        .unwrap()
                        .fixed_offset(),
                    10.00f32
                )
            ]
        );
    }
}
