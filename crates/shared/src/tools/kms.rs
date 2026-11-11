/*  Copyright 2022-23, Juspay India Pvt Ltd
    This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License
    as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This program
    is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY
    or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more details. You should have received a copy of
    the GNU Affero General Public License along with this program. If not, see <https://www.gnu.org/licenses/>.
*/

use aws_sdk_kms::primitives::Blob;
use aws_sdk_kms::Client;
use base64::Engine;
use error_stack::{report, Result};
use regex::Regex;
use thiserror::Error;

pub const KMS_PREFIX: &str = "kms:";

#[derive(Error, Debug)]
pub enum KmsError {
    #[error("Invalid kms: value: {0}")]
    InvalidValue(String),

    #[error("AWS KMS Decrypt failed: {0}")]
    DecryptError(String),
}

pub async fn decrypt_kms_text(text: &str) -> Result<String, KmsError> {
    if !text.starts_with(KMS_PREFIX) {
        return Ok(text.to_string());
    }
    let client = new_client().await;
    decrypt_with(&client, text).await
}

pub async fn decrypt_kms_in_dhall_text(text: &str) -> Result<String, KmsError> {
    let literals = find_kms_literals(text)?;
    if literals.is_empty() {
        return Ok(text.to_string());
    }
    let client = new_client().await;
    let mut replacements = Vec::with_capacity(literals.len());
    for (start, end, value) in literals {
        let plain = decrypt_with(&client, &value).await?;
        replacements.push((start, end, format!("\"{}\"", escape_dhall(&plain))));
    }
    Ok(apply_replacements(text, &replacements))
}

async fn new_client() -> Client {
    let config = aws_config::from_env().load().await;
    Client::new(&config)
}

async fn decrypt_with(client: &Client, text: &str) -> Result<String, KmsError> {
    let payload = text.strip_prefix(KMS_PREFIX).unwrap_or(text);
    let ciphertext = base64::engine::general_purpose::STANDARD
        .decode(payload.trim())
        .map_err(|_| {
            report!(KmsError::InvalidValue(
                "value after 'kms:' is not valid base64".to_string()
            ))
        })?;

    let response = client
        .decrypt()
        .ciphertext_blob(Blob::new(ciphertext))
        .send()
        .await
        .map_err(|err| report!(KmsError::DecryptError(format!("{:?}", err))))?;

    let plaintext = response
        .plaintext()
        .ok_or_else(|| report!(KmsError::DecryptError("no plaintext returned".to_string())))?;

    String::from_utf8(plaintext.as_ref().to_vec()).map_err(|_| {
        report!(KmsError::InvalidValue(
            "decrypted value is not valid UTF-8".to_string()
        ))
    })
}

fn find_kms_literals(text: &str) -> Result<Vec<(usize, usize, String)>, KmsError> {
    let re = Regex::new(r#""(kms:[A-Za-z0-9+/=\s]+)""#)
        .map_err(|err| report!(KmsError::InvalidValue(err.to_string())))?;
    Ok(re
        .captures_iter(text)
        .filter_map(|cap| {
            let whole = cap.get(0)?;
            let inner = cap.get(1)?;
            Some((whole.start(), whole.end(), inner.as_str().to_string()))
        })
        .collect())
}

fn apply_replacements(text: &str, replacements: &[(usize, usize, String)]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for (start, end, replacement) in replacements {
        out.push_str(&text[last..*start]);
        out.push_str(replacement);
        last = *end;
    }
    out.push_str(&text[last..]);
    out
}

fn escape_dhall(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '$' => out.push_str("\\u0024"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn text_without_prefix_is_returned_as_is() {
        let out = decrypt_kms_text("plain-password").await;
        assert_eq!(out.ok().as_deref(), Some("plain-password"));
    }

    #[tokio::test]
    async fn dhall_without_kms_values_never_calls_kms() {
        let text = "{ a = \"x\", b = 5 }";
        let out = decrypt_kms_in_dhall_text(text).await;
        assert_eq!(out.ok().as_deref(), Some(text));
    }

    #[test]
    fn finds_only_kms_literals() {
        let text = "{ a = \"kms:QUJD\", b = \"plain\", c = \"kms:REVG\" }";
        let found = find_kms_literals(text).ok().unwrap_or_default();
        let values: Vec<&str> = found.iter().map(|(_, _, v)| v.as_str()).collect();
        assert_eq!(values, vec!["kms:QUJD", "kms:REVG"]);
    }

    #[test]
    fn replaces_literals_in_place() {
        let text = "{ a = \"kms:QUJD\", b = \"plain\" }";
        let found = find_kms_literals(text).ok().unwrap_or_default();
        let replacements: Vec<_> = found
            .into_iter()
            .map(|(s, e, _)| (s, e, "\"secret\"".to_string()))
            .collect();
        assert_eq!(
            apply_replacements(text, &replacements),
            "{ a = \"secret\", b = \"plain\" }"
        );
    }

    #[test]
    fn escapes_dhall_special_characters() {
        assert_eq!(escape_dhall("a\"b\\c$d"), "a\\\"b\\\\c\\u0024d");
        assert_eq!(escape_dhall("p@ss*"), "p@ss*");
    }
}
