use std::{
    backtrace::Backtrace,
    fs::{self, OpenOptions},
    io::Write,
    panic,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

const LOG_FILE_NAME: &str = "diagnostics.log";
const MAX_LOG_BYTES: u64 = 256 * 1024;
const MAX_ENTRY_BYTES: usize = 8 * 1024;

static LOG_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();
static WRITE_LOCK: Mutex<()> = Mutex::new(());

pub(super) fn initialize() {
    let path = oziclock_storage::settings_path()
        .ok()
        .and_then(|path| path.parent().map(|directory| directory.join(LOG_FILE_NAME)));
    let _ = LOG_PATH.set(path);
    record("application-start");
}

pub(crate) fn install_panic_hook() {
    panic::set_hook(Box::new(|panic| {
        let location = panic
            .location()
            .map(|location| format!("{}:{}", location.file(), location.line()))
            .unwrap_or_else(|| "unknown-location".to_owned());
        let payload = if let Some(message) = panic.payload().downcast_ref::<&str>() {
            *message
        } else if let Some(message) = panic.payload().downcast_ref::<String>() {
            message.as_str()
        } else {
            "non-string panic payload"
        };
        record(&format!(
            "panic location={location} message={} backtrace={}",
            sanitize(payload),
            sanitize(&Backtrace::force_capture().to_string())
        ));
    }));
}

pub(super) fn record(event: &str) {
    let Some(path) = LOG_PATH.get().and_then(|path| path.as_deref()) else {
        return;
    };
    let Ok(_guard) = WRITE_LOCK.lock() else {
        return;
    };
    let _ = append(path, &format!("{} {event}\n", timestamp()));
}

fn append(path: &Path, entry: &str) -> std::io::Result<()> {
    if fs::metadata(path).is_ok_and(|metadata| metadata.len() >= MAX_LOG_BYTES) {
        fs::write(path, "")?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(&entry.as_bytes()[..entry.len().min(MAX_ENTRY_BYTES)])
}

fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| format!("unix-ms={}", duration.as_millis()))
        .unwrap_or_else(|_| "unix-ms=before-epoch".to_owned())
}

fn sanitize(value: &str) -> String {
    value.replace(['\n', '\r'], " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bl_082_log_entries_are_single_line_and_bounded() {
        let root = std::env::temp_dir().join(format!(
            "oziclock-diagnostics-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time is after Unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temporary directory is created");
        let path = root.join(LOG_FILE_NAME);
        fs::write(&path, vec![b'x'; MAX_LOG_BYTES as usize]).expect("full log is created");
        append(&path, "replacement\n").expect("oversized log is rotated");
        assert_eq!(
            fs::read_to_string(&path).expect("log is readable"),
            "replacement\n"
        );
        assert_eq!(sanitize("first\nsecond\rthird"), "first second third");
        fs::remove_dir_all(root).expect("temporary directory is removed");
    }
}
