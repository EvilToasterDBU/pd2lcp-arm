use serde::{Deserialize, Serialize};

use crate::error::{Error, ResultContextExt};

pub const METADATA_ENDPOINT: &str = "https://pd2-client-files.projectdiablo2.com";

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GameFilesMetadataRaw {
    checksum: Vec<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum LocalMetadataFormat {
    Official(GameFilesMetadataRaw),
    Legacy(Vec<GameFileMetadata>),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GameFileMetadata {
    pub checksum: String,
    pub name: String,
    pub url: String,
}

impl TryFrom<GameFilesMetadataRaw> for Vec<GameFileMetadata> {
    type Error = Error;

    fn try_from(value: GameFilesMetadataRaw) -> Result<Self, Self::Error> {
        value
            .checksum
            .iter()
            .map(|i| {
                let mut parts = i.split("  ");

                let checksum = parts
                    .next()
                    .ok_or(Error::InvalidMetadata)?
                    .trim()
                    .to_string();
                let name = parts.last().ok_or(Error::InvalidMetadata)?.trim();

                Ok(GameFileMetadata {
                    checksum,
                    name: name.to_string(),
                    url: format!("{}/{}", METADATA_ENDPOINT, name),
                })
            })
            .collect::<Result<Vec<GameFileMetadata>, Error>>()
    }
}

impl From<&[GameFileMetadata]> for GameFilesMetadataRaw {
    fn from(files: &[GameFileMetadata]) -> Self {
        Self {
            checksum: files
                .iter()
                .map(|file| format!("{}  {}", file.checksum, file.name))
                .collect(),
        }
    }
}

pub fn parse_local_metadata(data: &[u8]) -> Result<Vec<GameFileMetadata>, crate::error::Error> {
    let parsed: LocalMetadataFormat =
        serde_json::from_slice(data).context("Failed to parse local metadata")?;

    match parsed {
        LocalMetadataFormat::Official(raw) => raw.try_into(),
        LocalMetadataFormat::Legacy(files) => Ok(files),
    }
}

pub async fn get_metadata() -> Result<Vec<GameFileMetadata>, Error> {
    let metadata_url = format!("{}/metadata.json", METADATA_ENDPOINT);

    let text = reqwest::get(metadata_url)
        .await
        .context("Failed to download metadata. Are you connected to the internet?")?
        .text()
        .await?;

    let metadata_raw: GameFilesMetadataRaw =
        serde_json::from_str(&text).context("Failed to parse metadata")?;

    metadata_raw.try_into()
}

#[cfg(test)]
mod tests {
    use super::{GameFileMetadata, GameFilesMetadataRaw, parse_local_metadata};

    #[test]
    fn parses_official_checksum_manifest() {
        let data = br#"{"checksum":["0123456789abcdef0123456789abcdef  Game.exe"]}"#;

        let files = parse_local_metadata(data).unwrap();

        assert_eq!(
            files,
            vec![GameFileMetadata {
                checksum: "0123456789abcdef0123456789abcdef".to_string(),
                name: "Game.exe".to_string(),
                url: "https://pd2-client-files.projectdiablo2.com/Game.exe".to_string(),
            }]
        );
    }

    #[test]
    fn keeps_reading_previous_pd2lcp_manifest() {
        let data = br#"[{"checksum":"0123456789abcdef0123456789abcdef","name":"Game.exe","url":"https://pd2-client-files.projectdiablo2.com/Game.exe"}]"#;

        let files = parse_local_metadata(data).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "Game.exe");
    }

    #[test]
    fn serializes_back_to_the_official_manifest_format() {
        let files = vec![GameFileMetadata {
            checksum: "0123456789abcdef0123456789abcdef".to_string(),
            name: "Game.exe".to_string(),
            url: "https://pd2-client-files.projectdiablo2.com/Game.exe".to_string(),
        }];
        let raw = GameFilesMetadataRaw::from(files.as_slice());
        let serialized = serde_json::to_vec(&raw).unwrap();

        assert_eq!(
            serialized,
            br#"{"checksum":["0123456789abcdef0123456789abcdef  Game.exe"]}"#
        );
    }
}
