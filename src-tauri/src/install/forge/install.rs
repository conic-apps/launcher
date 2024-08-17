use std::{io::BufRead, path::PathBuf, process::Stdio};

use tokio::io::AsyncWriteExt;

use crate::{folder::MinecraftLocation, platform::DELIMITER, DATA_LOCATION, HTTP_CLIENT};

/// Forge Install Bootstrapper - By bangbang93
/// [Github Repo](https://github.com/bangbang93/forge-install-bootstrapper)
static FORGE_INSTALL_BOOTSTRAPPER: &[u8] = include_bytes!("./forge-install-bootstrapper.jar");

/// Forge Install Bootstrapper - SteveXMH's Fork
///
/// [Github Repo](https://github.com/Steve-xmh/forge-install-bootstrapper)
///
/// A modified version based on [bangbang93/forge-install-bootstrapper](https://github.com/bangbang93/forge-install-bootstrapper).
/// The purpose is to support automated installation of all versions of the installer (any version since 1.5.2 that provides an installer)
///
/// Usage: java -cp "forge-install-bootstrapper.jar:forge-xxx-installer.jar" com.bangbang93.ForgeInstaller "PathToDotMinecraft"
static FORGE_INSTALL_BOOTSTRAPPER_OLD: &[u8] = include_bytes!("./forge-install-bootstrapper.jar");

pub async fn install(
    install_dir: &PathBuf,
    forge_version: &str,
    mcversion: &str,
) -> anyhow::Result<()> {
    let installer_path = download_installer(mcversion, forge_version).await?;
    let bootstrapper_path = DATA_LOCATION
        .get()
        .unwrap()
        .temp
        .join("forge-install-bootstrapper.jar");
    tokio::fs::write(&bootstrapper_path, FORGE_INSTALL_BOOTSTRAPPER).await?;
    println!(
        "\"{}:{}\"",
        bootstrapper_path.to_str().unwrap(),
        installer_path.to_str().unwrap()
    );
    let java = DATA_LOCATION.get().unwrap().default_jre.clone();
    let mut command = std::process::Command::new(java)
        .arg("-cp")
        .arg(format!(
            "{}{}{}",
            bootstrapper_path.to_str().unwrap(),
            DELIMITER,
            installer_path.to_str().unwrap()
        ))
        .arg("com.bangbang93.ForgeInstaller")
        .arg(install_dir)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    println!("Running forge installer");
    let out = command.stdout.take().unwrap();
    let mut out = std::io::BufReader::new(out);
    let mut buf = String::new();
    let mut success = false;
    while let Ok(_) = out.read_line(&mut buf) {
        if let Ok(Some(_)) = command.try_wait() {
            break;
        }
        if buf.ends_with("\ntrue\n") {
            success = true;
            println!("Install success!")
        }
        println!("{}", buf);
    }
    let output = command.wait_with_output().unwrap();
    if !success || !output.status.success() {
        tokio::fs::remove_file(installer_path).await?;
        return Err(anyhow::Error::msg("Forge installer failed"));
    }
    tokio::fs::remove_file(installer_path).await?;
    Ok(())
}

async fn download_installer(mcversion: &str, forge_version: &str) -> anyhow::Result<PathBuf> {
    let installer_url  = format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{mcversion}-{forge_version}/forge-{mcversion}-{forge_version}-installer.jar");
    let installer_path = DATA_LOCATION
        .get()
        .unwrap()
        .temp
        .join("forge-installer.jar");
    tokio::fs::create_dir_all(
        installer_path
            .parent()
            .ok_or(anyhow::Error::msg("Unknown Error"))?,
    )
    .await?;
    let mut file = tokio::fs::File::create(&installer_path).await?;
    let response = HTTP_CLIENT.get().unwrap().get(installer_url).send().await?;
    let src = response.bytes().await?;
    file.write_all(&src).await?;
    Ok(installer_path)
}
