use crate::winapi::Files;
use log::{Metadata, Record};
use std::fs::File;
use std::io::{Error as IoError, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use windows::Win32::Foundation::SYSTEMTIME;

#[cfg(not(test))]
use crate::winapi::get_local_time;

pub struct FileLogger {
    inner: Mutex<Inner>,
}

struct Inner {
    buffer: Vec<u8>,
    file: Option<File>,
}

const MAX_LOG_FILES: usize = 10;
const LOG_FILENAME_PATTERN: &str = "LilPowerMan????????_???.log";

fn format_log_filename_prefix(time: &SYSTEMTIME) -> String {
    format!(
        "LilPowerMan{:04}{:02}{:02}_",
        time.wYear, time.wMonth, time.wDay
    )
}

fn format_log_filename(prefix: &str, counter: i32) -> String {
    format!("{}{:03}.log", prefix, counter)
}

impl FileLogger {
    fn new_log_file(path: &Path) -> Result<File, IoError> {
        // find existing log files
        let mut path = PathBuf::from(path);
        path.push(LOG_FILENAME_PATTERN);
        let existing_logs: Result<Vec<_>, _> = Files::find(path.as_os_str()).collect();
        path.pop();
        let mut existing_logs = existing_logs?;
        existing_logs.sort_unstable();
        existing_logs.reverse(); // newest files first
        let existing_logs = existing_logs;

        // delete old log files
        let mut deleted = 0;
        for log in existing_logs.iter().skip(MAX_LOG_FILES - 1) {
            path.push(log);
            if let Err(err) = Files::delete(path.as_os_str()) {
                warn!(
                    "Failed to delete log file {}: {}",
                    log.to_string_lossy(),
                    err
                );
            } else {
                deleted += 1;
            }
            path.pop();
        }
        if deleted > 0 {
            info!("Deleted {} old log files", deleted);
        }

        // extract last counter
        let time = get_local_time();
        let mut counter = 0;
        let prefix = format_log_filename_prefix(&time);
        for log in existing_logs.iter().take(MAX_LOG_FILES - 1) {
            let log = log.to_string_lossy();
            if let Some(suffix) = log.strip_prefix(&prefix) {
                // SAFETY: Filename pattern should enforce suffix length
                if let Ok(i) = suffix[..3].parse::<i32>() {
                    if i == 999 {
                        warn!("Log filename counter overflow, resetting to zero");
                        counter = 0;
                    } else {
                        counter = i + 1;
                    }
                    break;
                } else {
                    debug!(
                        "Unexpected log filename counter suffix: {}, skipping",
                        suffix
                    );
                }
            }
        }

        path.push(format_log_filename(&prefix, counter));
        Files::create(path.as_os_str())
    }

    pub fn new() -> Self {
        FileLogger {
            inner: Mutex::new(Inner {
                buffer: Vec::new(),
                file: None,
            }),
        }
    }

    pub fn init(&self, path: &Path) -> Result<(), IoError> {
        let mut new_log = Self::new_log_file(path)?;
        let mut inner = self.inner.lock().unwrap();
        new_log.write_all(&std::mem::replace(&mut inner.buffer, Vec::new()))?;
        inner.file = Some(new_log);
        Ok(())
    }
}

impl log::Log for FileLogger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        let time = get_local_time();
        let s = format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}[{}][{}] {}\n",
            time.wYear,
            time.wMonth,
            time.wDay,
            time.wHour,
            time.wMinute,
            time.wSecond,
            time.wMilliseconds,
            record.level(),
            record.target(),
            record.args()
        );
        let mut inner = self.inner.lock().unwrap();
        if let Some(file) = &mut inner.file {
            _ = file.write_all(s.as_bytes());
        } else {
            inner.buffer.extend_from_slice(s.as_bytes());
        }
    }

    fn flush(&self) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(f) = &mut inner.file {
            if let Err(err) = f.sync_data() {
                error!("Failed to flush log file to disk: {}", err);
            }
        }
    }
}

#[cfg(test)]
fn get_local_time() -> SYSTEMTIME {
    SYSTEMTIME {
        wYear: 2025,
        wMonth: 05,
        wDayOfWeek: 0,
        wDay: 10,
        wHour: 23,
        wMinute: 15,
        wSecond: 46,
        wMilliseconds: 788,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::{Level, Log};
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn id() -> usize {
        static ID: AtomicUsize = AtomicUsize::new(0);
        ID.fetch_add(1, Ordering::SeqCst)
    }

    fn prepare_dir(files: Vec<&str>) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("LilPowerMan-UnitTests-{}", id()));
        _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir(path.as_os_str()).expect("Failed to create dir");
        for file in files {
            path.push(file);
            std::fs::write(&path, "Hello, tests!").expect("Failed to create test file");
            path.pop();
        }
        path
    }

    fn log(path: &Path, s: &str) {
        let logger = FileLogger::new();
        logger.init(path).expect("Failed to initialize logger");
        logger.log(
            &Record::builder()
                .level(Level::Info)
                .target("tests")
                .file(Some("logging.rs"))
                .args(format_args!("Hello, {}!", s))
                .line(Some(100))
                .build(),
        );
    }

    fn assert_file(path: &Path, contents: &str) {
        let actual = std::fs::read(&path).expect("Failed to read file");
        let actual = String::from_utf8(actual).expect("File contents are not valid UTF-8");
        assert_eq!(actual, contents);
    }

    #[test]
    fn clean_start() {
        // Arrange
        let mut path = prepare_dir(vec![]);

        // Act
        log(&path, "Clean start");

        // Assert
        path.push("LilPowerMan20250510_000.log");
        assert_file(
            &path,
            "2025-05-10T23:15:46.788[INFO][tests] Hello, Clean start!\n",
        );
    }

    #[test]
    fn existing_log() {
        // Arrange
        let mut path = prepare_dir(vec!["LilPowerMan20250510_173.log"]);

        // Act
        log(&path, "Existing log");

        // Assert
        path.push("LilPowerMan20250510_174.log");
        assert_file(
            &path,
            "2025-05-10T23:15:46.788[INFO][tests] Hello, Existing log!\n",
        );
    }

    #[test]
    fn other_dates() {
        // Arrange
        let mut path = prepare_dir(vec![
            "LilPowerMan20250509_784.log",
            "LilPowerMan20250510_173.log",
            "LilPowerMan20250511_044.log",
        ]);

        // Act
        log(&path, "Existing log");

        // Assert
        path.push("LilPowerMan20250510_174.log");
        assert_file(
            &path,
            "2025-05-10T23:15:46.788[INFO][tests] Hello, Existing log!\n",
        );
    }

    #[test]
    fn skip_non_numeric() {
        // Arrange
        let mut path = prepare_dir(vec![
            "LilPowerMan20250510_003.log",
            "LilPowerMan20250510_xyz.log",
        ]);

        // Act
        log(&path, "Skip non-numeric");

        // Assert
        path.push("LilPowerMan20250510_004.log");
        assert_file(
            &path,
            "2025-05-10T23:15:46.788[INFO][tests] Hello, Skip non-numeric!\n",
        );
    }

    #[test]
    fn overflow() {
        // Arrange
        let mut path = prepare_dir(vec!["LilPowerMan20250510_999.log"]);

        // Act
        log(&path, "Overflow");

        // Assert
        path.push("LilPowerMan20250510_000.log");
        assert_file(
            &path,
            "2025-05-10T23:15:46.788[INFO][tests] Hello, Overflow!\n",
        );
    }

    #[test]
    fn delete_old_files() {
        // Arrange
        let mut path = prepare_dir(vec![
            "LilPowerMan20250509_000.log",
            "LilPowerMan20250509_001.log",
            "LilPowerMan20250509_002.log",
            "LilPowerMan20250509_003.log",
            "LilPowerMan20250509_004.log",
            "LilPowerMan20250510_000.log",
            "LilPowerMan20250510_001.log",
            "LilPowerMan20250510_002.log",
            "LilPowerMan20250510_003.log",
            "LilPowerMan20250510_004.log",
        ]);

        // Act
        log(&path, "Delete old files");

        // Assert
        path.push("LilPowerMan20250510_005.log");
        assert_file(
            &path,
            "2025-05-10T23:15:46.788[INFO][tests] Hello, Delete old files!\n",
        );
        path.pop();
        path.push("LilPowerMan20250509_000.log");
        assert!(!std::fs::exists(&path).expect("Failed to check file existence"));
    }

    #[test]
    fn skip_locked_files() {
        // Arrange
        let mut path = prepare_dir(vec![
            "LilPowerMan20250509_001.log",
            "LilPowerMan20250509_002.log",
            "LilPowerMan20250509_003.log",
            "LilPowerMan20250509_004.log",
            "LilPowerMan20250510_000.log",
            "LilPowerMan20250510_001.log",
            "LilPowerMan20250510_002.log",
            "LilPowerMan20250510_003.log",
            "LilPowerMan20250510_004.log",
        ]);
        path.push("LilPowerMan20250509_000.log");
        let file = Files::create(path.as_os_str()).expect("Failed to create test file");
        path.pop();

        // Act
        log(&path, "Skip locked files");

        // Assert
        path.push("LilPowerMan20250510_005.log");
        assert_file(
            &path,
            "2025-05-10T23:15:46.788[INFO][tests] Hello, Skip locked files!\n",
        );
        path.pop();
        path.push("LilPowerMan20250509_000.log");
        assert!(std::fs::exists(&path).expect("Failed to check file existence"));
        drop(file); // Ensure the file is open during the entire test
    }
}
