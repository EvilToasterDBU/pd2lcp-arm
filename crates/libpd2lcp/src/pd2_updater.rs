use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};

use crate::{
    error::{Error, ResultContextExt},
    event::{Event, EventNotify},
    metadata::{GameFilesMetadataRaw, get_metadata, parse_local_metadata},
    state::State,
    utils::compute_hash,
};

pub async fn update_available(state: Option<State>) -> Result<bool, Error> {
    let state = state.ok_or(Error::Pd2lcpNotInitialised)?;

    let remote_metadata = get_metadata().await?;

    let local_metadata_path = state.pd2_dir().join("local_metadata.json");

    if !local_metadata_path.is_file() {
        return Ok(true);
    }

    let mut local_metadata_file = File::open(&local_metadata_path)
        .await
        .context("Failed to open local metadata")?;

    let mut buffer = Vec::new();

    local_metadata_file
        .read_to_end(&mut buffer)
        .await
        .context("Failed to read local metadata")?;

    let local_metadata =
        parse_local_metadata(&buffer).context("Failed to deserialise local metadata")?;

    if local_metadata != remote_metadata {
        return Ok(true);
    }

    Ok(false)
}

pub async fn install_pd2(state: Option<State>, notify: EventNotify) -> Result<(), Error> {
    let state = state.ok_or(Error::Pd2lcpNotInitialised)?;

    let metadata_game_files = get_metadata().await?;

    let pd2_files = state.pd2_dir();

    if !pd2_files.is_dir() {
        tokio::fs::create_dir_all(&pd2_files).await?;
    }

    let total = metadata_game_files.len() as u32;

    let mut done: i64 = -1;

    for file_metadata in metadata_game_files.iter() {
        done += 1;

        notify.notify(Event::UpdatingPD2 {
            done: done as u32,
            total,
        })?;

        let output_file_path = pd2_files.join(&file_metadata.name);

        let needs_update = match tokio::fs::read(&output_file_path).await {
            Ok(data) => compute_hash(&data)? != file_metadata.checksum,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
            Err(error) => return Err(error.into()),
        };

        if needs_update {
            let data = reqwest::get(&file_metadata.url)
                .await
                .context("Failed to download pd2 file")?
                .error_for_status()
                .context("PD2 file server returned an error status")?
                .bytes()
                .await?;

            let actual_hash = compute_hash(&data)?;
            if actual_hash != file_metadata.checksum {
                return Err(Error::FileChecksumMismatch {
                    file: file_metadata.name.clone(),
                    expected: file_metadata.checksum.clone(),
                    actual: actual_hash,
                });
            }

            let file_name = output_file_path
                .file_name()
                .expect("PD2 update paths always have a file name")
                .to_string_lossy();
            let temporary_file_path =
                output_file_path.with_file_name(format!("{file_name}.pd2lcp-update.tmp"));
            let mut temporary_file = File::create(&temporary_file_path)
                .await
                .context("Failed to create temporary PD2 file")?;
            temporary_file
                .write_all(&data)
                .await
                .context("Failed to write temporary PD2 file")?;
            temporary_file.flush().await?;
            drop(temporary_file);

            if output_file_path.exists() {
                tokio::fs::remove_file(&output_file_path).await?;
            }
            tokio::fs::rename(&temporary_file_path, &output_file_path).await?;
        }
    }

    // Now we write to the local_metadata.json file
    let local_metadata_path = pd2_files.join("local_metadata.json");

    let mut local_metadata_file = File::create(local_metadata_path)
        .await
        .context("Failed to create local metadata file")?;

    let metadata_raw = GameFilesMetadataRaw::from(metadata_game_files.as_slice());
    let serialised =
        serde_json::to_vec_pretty(&metadata_raw).context("Failed to serialise metadata")?;

    local_metadata_file
        .write_all(&serialised)
        .await
        .context("Failed to write local metadata file")?;

    notify.notify(Event::DoneUpdating)?;

    Ok(())
}
