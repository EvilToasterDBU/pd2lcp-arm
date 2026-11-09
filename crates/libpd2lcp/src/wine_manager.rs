use tokio::{
    fs::{File, create_dir_all, read_dir, rename},
    io::AsyncWriteExt,
    process::Command,
};

use crate::{
    error::{Error, ResultContextExt},
    state::State,
};

const WINE_ARCHIVE_NAME: &str = if cfg!(target_arch = "aarch64") {
    "wine-11.14-staging-tkg-amd64-wow64"
} else {
    "wine-11.14-staging-tkg-amd64"
};
pub const WINE_URL: &str = if cfg!(target_arch = "aarch64") {
    "https://github.com/Kron4ek/Wine-Builds/releases/download/11.14/wine-11.14-staging-tkg-amd64-wow64.tar.xz"
} else {
    "https://github.com/Kron4ek/Wine-Builds/releases/download/11.14/wine-11.14-staging-tkg-amd64.tar.xz"
};

pub async fn fetch_wine(state: &State) -> Result<(), Error> {
    let install_path = state.wine_dir()?;

    create_dir_all(install_path).await?;

    let wine_bytes = reqwest::get(WINE_URL)
        .await
        .context("Failed to download Wine. Check your internet.")?
        .bytes()
        .await?;

    let wine_archive_path = install_path.join("wine_tarball.tar.xz");

    let mut wine_archive_file = File::create(&wine_archive_path)
        .await
        .context("Failed to create wine archive")?;

    wine_archive_file
        .write_all(&wine_bytes)
        .await
        .context("Failed to write wine archive")?;

    // Now we untar it
    let status = Command::new("tar")
        .arg("-xf")
        .arg(wine_archive_path)
        .arg("-C")
        .arg(install_path)
        .status()
        .await
        .context("Do you have tar installed?")?;

    if !status.success() {
        return Err(Error::FailedToUntarArchive);
    }

    while let Some(file) = read_dir(install_path.join(WINE_ARCHIVE_NAME))
        .await?
        .next_entry()
        .await?
    {
        rename(file.path(), install_path.join(file.file_name())).await?;
    }

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    unpack_box64_libraries(state).await?;

    Ok(())
}

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
async fn unpack_box64_libraries(state: &State) -> Result<(), Error> {
    let Some(archive) = state.box64_libs_archive() else {
        return Ok(());
    };

    let install_path = state.wine_dir()?.join("box64-libs");
    create_dir_all(&install_path).await?;

    let status = Command::new("tar")
        .arg("-xzf")
        .arg(archive)
        .arg("-C")
        .arg(install_path)
        .status()
        .await
        .context("Failed to extract Box64 libraries")?;

    if !status.success() {
        return Err(Error::FailedToUntarArchive);
    }

    Ok(())
}

pub async fn create_prefix(state: &State) -> Result<(), Error> {
    let prefix = state.wine_prefix()?;

    create_dir_all(prefix).await?;

    let status = state
        .wine_command("wineboot")?
        .env("WINEPREFIX", prefix)
        .arg("-i")
        .status()
        .await?;

    if !status.success() {
        return Err(Error::WineInitFailed);
    }

    // wine reg add "HKCU\\Software\\Wine" /v Version /d win7 /f
    let status_set_win7 = state
        .wine_command("wine")?
        .env("WINEPREFIX", prefix)
        .args([
            "reg",
            "add",
            "HKCU\\Software\\Wine",
            "/v",
            "Version",
            "/d",
            "win7",
            "/f",
        ])
        .status()
        .await?;

    if !status_set_win7.success() {
        return Err(Error::WineInitFailed);
    }

    tokio::fs::symlink(state.game_dir(), state.wine_dosdevice("a:")?)
        .await
        .context("Failed to symlink games folder -> a: dosdevice")?;

    // Now we kill wineserver to update dosdevices
    let status_wineserver = state
        .wine_command("wineserver")?
        .env("WINEPREFIX", prefix)
        .arg("-k")
        .status()
        .await?;

    if !status_wineserver.success() {
        return Err(Error::WineInitFailed);
    }

    Ok(())
}
