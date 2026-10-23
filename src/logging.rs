//! 日志系统：终端输出 + 按天滚动的文件日志。
//!
//! 日志默认写入 `~/.screenscan/logs/screenscan-YYYY-MM-DD.log`，
//! 自动保留最近 [`RETENTION_DAYS`] 天，便于用户与开发者排查问题。

use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;

use chrono::Local;
use chrono::NaiveDate;
use log::Log;
use log::Metadata;
use log::Record;

use crate::config::LogLevel;
use crate::util::path;

const LOG_PREFIX: &str = "screenscan-";
const RETENTION_DAYS: u64 = 14;

pub fn log_dir() -> PathBuf {
    path::home_dir().join(".screenscan").join("logs")
}

pub fn init(level: LogLevel) {
    let mut builder = env_logger::Builder::from_env(
        env_logger::Env::default()
            .default_filter_or("warn,ScreenScan=trace,libwayshot_xcap=off,zbus=warn,tracing=warn"),
    );
    builder.format_timestamp_millis();
    let console = builder.build();
    let env_max = console.filter();

    let logger = TeeLogger {
        console,
        file: FileLogger::new(log_dir()),
    };

    if let Err(e) = log::set_boxed_logger(Box::new(logger)) {
        eprintln!("初始化日志系统失败: {e}");
        return;
    }

    // RUST_LOG 优先；否则由设置中的日志级别控制终端与文件日志。
    if std::env::var_os("RUST_LOG").is_some() {
        log::set_max_level(env_max);
    } else {
        log::set_max_level(level.filter());
    }
}

/// 打开日志目录（失败时返回错误，由调用方提示）。
pub fn open_log_dir() -> std::io::Result<()> {
    let dir = log_dir();
    std::fs::create_dir_all(&dir)?;

    let program = if cfg!(target_os = "linux") {
        "xdg-open"
    } else if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "explorer"
    } else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "当前平台不支持打开日志目录",
        ));
    };

    std::process::Command::new(program)
        .arg(&dir)
        .spawn()
        .map(|_| ())
}

struct TeeLogger {
    console: env_logger::Logger,
    file: FileLogger,
}

impl Log for TeeLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        self.console.enabled(metadata)
    }

    fn log(&self, record: &Record<'_>) {
        if self.console.enabled(record.metadata()) {
            self.console.log(record);
            self.file.write(record);
        }
    }

    fn flush(&self) {
        self.console.flush();
    }
}

struct FileLogger {
    dir: PathBuf,
    state: Mutex<Option<LogFile>>,
}

struct LogFile {
    date: NaiveDate,
    file: File,
}

impl FileLogger {
    fn new(dir: PathBuf) -> Self {
        if let Err(e) = std::fs::create_dir_all(&dir) {
            eprintln!("无法创建日志目录 {}: {e}", dir.display());
        } else {
            cleanup_old_logs(&dir);
        }

        let today = Local::now().date_naive();
        let state = match open_log_file(&dir, today) {
            Ok(file) => Some(LogFile { date: today, file }),
            Err(e) => {
                eprintln!("无法创建日志文件: {e}");
                None
            }
        };

        Self {
            dir,
            state: Mutex::new(state),
        }
    }

    fn write(&self, record: &Record<'_>) {
        let now = Local::now();
        let today = now.date_naive();
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        if state.as_ref().is_none_or(|log| log.date != today) {
            match open_log_file(&self.dir, today) {
                Ok(file) => {
                    *state = Some(LogFile { date: today, file });
                }
                Err(e) => {
                    eprintln!("无法写入日志文件: {e}");
                    *state = None;
                    return;
                }
            }
        }

        if let Some(log) = state.as_mut() {
            let location = match (record.file(), record.line()) {
                (Some(file), Some(line)) => format!("{file}:{line}"),
                (Some(file), None) => file.to_owned(),
                _ => record.target().to_owned(),
            };
            let _ = writeln!(
                log.file,
                "{} [{:<5}] {} ({}): {}",
                now.format("%Y-%m-%d %H:%M:%S%.3f"),
                record.level().as_str(),
                record.target(),
                location,
                record.args()
            );
            let _ = log.file.flush();
        }
    }
}

fn open_log_file(dir: &Path, date: NaiveDate) -> std::io::Result<File> {
    std::fs::create_dir_all(dir)?;
    let name = format!("{}{}.log", LOG_PREFIX, date.format("%Y-%m-%d"));
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(name))
}

fn cleanup_old_logs(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let Some(cutoff) = std::time::SystemTime::now().checked_sub(std::time::Duration::from_secs(
        60 * 60 * 24 * RETENTION_DAYS,
    )) else {
        return;
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with(LOG_PREFIX) || !name.ends_with(".log") {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        if modified < cutoff {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_record_to_daily_file() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "screenscan-log-test-{}-{unique}",
            std::process::id()
        ));

        let logger = FileLogger::new(dir.clone());
        let record = log::Record::builder()
            .args(format_args!("hello diagnostics"))
            .level(log::Level::Info)
            .target("ScreenScan::test")
            .file(Some("test.rs"))
            .line(Some(42))
            .build();
        logger.write(&record);

        let file = dir.join(format!(
            "{}{}.log",
            LOG_PREFIX,
            Local::now().format("%Y-%m-%d")
        ));
        let text = std::fs::read_to_string(&file).expect("log file should exist");
        assert!(text.contains("hello diagnostics"));
        assert!(text.contains("INFO"));
        assert!(text.contains("ScreenScan::test"));
        assert!(text.contains("test.rs:42"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
