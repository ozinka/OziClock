#[cfg(target_os = "windows")]
use std::process::Command;
use std::{env, fs, io, path::PathBuf};

pub(super) fn set_enabled(enabled: bool) -> io::Result<()> {
    let executable = env::current_exe()?;
    #[cfg(target_os = "macos")]
    {
        let dir = dirs_path("Library/LaunchAgents")?;
        fs::create_dir_all(&dir)?;
        let path = dir.join("com.ozinka.oziclock.plist");
        if enabled {
            let content = format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?><plist version=\"1.0\"><dict><key>Label</key><string>com.ozinka.oziclock</string><key>ProgramArguments</key><array><string>{}</string></array><key>RunAtLoad</key><true/></dict></plist>",
                executable.display()
            );
            fs::write(path, content)?;
        } else if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }
    #[cfg(target_os = "linux")]
    {
        let dir = env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| dirs_path(".config").unwrap_or_default())
            .join("autostart");
        fs::create_dir_all(&dir)?;
        let path = dir.join("oziclock.desktop");
        if enabled {
            fs::write(
                path,
                format!(
                    "[Desktop Entry]\nType=Application\nName=OziClock\nExec={}\nX-GNOME-Autostart-enabled=true\n",
                    executable.display()
                ),
            )?;
        } else if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }
    #[cfg(target_os = "windows")]
    {
        let status = if enabled {
            Command::new("reg")
                .args([
                    "add",
                    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                    "OziClock",
                    "/d",
                    executable.to_string_lossy().as_ref(),
                    "/f",
                ])
                .status()?
        } else {
            Command::new("reg")
                .args([
                    "delete",
                    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                    "/v",
                    "OziClock",
                    "/f",
                ])
                .status()?
        };
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other(
                "failed to update Windows startup registry",
            ))
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = executable;
        let _ = enabled;
        Ok(())
    }
}

fn dirs_path(suffix: &str) -> io::Result<PathBuf> {
    env::var_os("HOME")
        .map(|home| PathBuf::from(home).join(suffix))
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is not set"))
}
