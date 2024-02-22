use std::{
    error::Error,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Mutex, RwLock},
};

use chrono::Utc;
use log::Level as LogLevel;

pub use log::debug;
pub use log::error;
pub use log::info;
pub use log::trace;
pub use log::warn;

const TIMESTAMP_FORMAT: &str = "%Y-%m-%dT%T%.3f";

type Size = u64;
type FileAge = u32;
const MIN_AS_MILLI_SEC: i64 = 60 * 1000;
const HOUR_AS_MILLI_SEC: i64 = 60 * MIN_AS_MILLI_SEC;
const DAY_AS_MILLI_SEC: i64 = 24 * HOUR_AS_MILLI_SEC;
const WEEK_AS_MILLI_SEC: i64 = 7 * DAY_AS_MILLI_SEC;
const MONTH_AS_MILLI_SEC: i64 = 30 * DAY_AS_MILLI_SEC;
const YEAR_AS_MILLI_SEC: i64 = 365 * DAY_AS_MILLI_SEC;

#[derive(Debug)]
pub enum Rotation {
    Minutely,
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Yearly,
    Never,
}

impl Rotation {
    fn next_rotation_time(&self) -> i64 {
        let curr_ts = Utc::now().timestamp_millis();
        match self {
            Self::Minutely => curr_ts + MIN_AS_MILLI_SEC,
            Self::Hourly => curr_ts + HOUR_AS_MILLI_SEC,
            Self::Daily => curr_ts + DAY_AS_MILLI_SEC,
            Self::Weekly => curr_ts + WEEK_AS_MILLI_SEC,
            Self::Monthly => curr_ts + MONTH_AS_MILLI_SEC,
            Self::Yearly => curr_ts + YEAR_AS_MILLI_SEC,
            Self::Never => 0,
        }
    }
}

#[derive(Debug)]
enum RotationRemove {
    ByMaxAge(FileAge),
    ByCount(u32),
}

#[derive(Debug)]
struct Builder<T, U, V, W> {
    log_level: LogLevel,
    file_path: T,
    rotation: Rotation,
    max_size: U,
    min_size: V,
    compress: bool,
    delay_compress: bool,
    if_empty: bool,
    rotation_remove: W,
}

struct NoFilePath;
struct NoMaxSize;
struct NoMinSize;
struct NoRotationRemove;

impl<T, U, V, W> Builder<T, U, V, W> {
    fn log_level(mut self, log_level: LogLevel) -> Self {
        self.log_level = log_level;
        self
    }
    fn rotation(mut self, rotation: Rotation) -> Self {
        self.rotation = rotation;
        self
    }
    fn compress(mut self, compress: bool) -> Self {
        self.compress = compress;
        self
    }
    fn delay_compress(mut self, delay_compress: bool) -> Self {
        self.delay_compress = delay_compress;
        self
    }
    fn if_empty(mut self, if_empty: bool) -> Self {
        self.if_empty = if_empty;
        self
    }
}

impl<T, U, V> Builder<T, U, V, NoRotationRemove> {
    fn rotation_count(self, count: u32) -> Builder<T, U, V, RotationRemove> {
        let rotation_remove = RotationRemove::ByCount(count);
        Builder {
            log_level: self.log_level,
            file_path: self.file_path,
            rotation: self.rotation,
            max_size: self.max_size,
            min_size: self.min_size,
            compress: self.compress,
            delay_compress: self.delay_compress,
            if_empty: self.if_empty,
            rotation_remove,
        }
    }
    fn max_age(self, age: FileAge) -> Builder<T, U, V, RotationRemove> {
        let rotation_remove = RotationRemove::ByMaxAge(age);
        Builder {
            log_level: self.log_level,
            file_path: self.file_path,
            rotation: self.rotation,
            max_size: self.max_size,
            min_size: self.min_size,
            compress: self.compress,
            delay_compress: self.delay_compress,
            if_empty: self.if_empty,
            rotation_remove,
        }
    }
}

impl<U, V, W> Builder<NoFilePath, U, V, W> {
    fn file_path(self, file_path: &str) -> Builder<PathBuf, U, V, W> {
        let file_path = PathBuf::from(file_path);
        Builder {
            log_level: self.log_level,
            file_path,
            rotation: self.rotation,
            max_size: self.max_size,
            min_size: self.min_size,
            compress: self.compress,
            delay_compress: self.delay_compress,
            if_empty: self.if_empty,
            rotation_remove: self.rotation_remove,
        }
    }
}

impl<T, W> Builder<T, NoMaxSize, NoMinSize, W> {
    fn max_size(self, max_size: Size) -> Builder<T, Size, NoMinSize, W> {
        Builder {
            log_level: self.log_level,
            file_path: self.file_path,
            rotation: self.rotation,
            max_size,
            min_size: self.min_size,
            compress: self.compress,
            delay_compress: self.delay_compress,
            if_empty: self.if_empty,
            rotation_remove: self.rotation_remove,
        }
    }
    fn min_size(self, min_size: Size) -> Builder<T, NoMaxSize, Size, W> {
        Builder {
            log_level: self.log_level,
            file_path: self.file_path,
            rotation: self.rotation,
            max_size: self.max_size,
            min_size,
            compress: self.compress,
            delay_compress: self.delay_compress,
            if_empty: self.if_empty,
            rotation_remove: self.rotation_remove,
        }
    }
}

impl<U, V> Builder<PathBuf, U, V, RotationRemove> {
    fn finish(self) -> Result<(), Box<dyn Error>> {
        let next_rotation_time = RwLock::new(self.rotation.next_rotation_time());
        let file_path = self.file_path;
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file_handle = File::options().create(true).append(true).open(file_path)?;
        let file_handle = Mutex::new(file_handle);
        let logger = Logger {
            log_level: self.log_level,
            file_handle,
            next_rotation_time,
        };
        log::set_max_level(self.log_level.to_level_filter());
        log::set_boxed_logger(Box::new(logger))?;
        Ok(())
    }
}

#[derive(Debug)]
struct Logger {
    log_level: LogLevel,
    file_handle: Mutex<File>,
    next_rotation_time: RwLock<i64>,
}

impl Logger {
    fn write_message(&self, message: &str) -> Result<(), Box<dyn Error + '_>> {
        let mut handle = self.file_handle.lock()?;
        handle.write_all(message.as_bytes())?;
        handle.flush()?;
        Ok(())
    }
    fn file_size(&self) -> Result<u64, Box<dyn Error + '_>> {
        let len = self.file_handle.lock()?.metadata()?.len();
        Ok(len)
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.log_level
    }
    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let file_line = match (record.file(), record.line()) {
            (Some(f), Some(l)) => format!("{}:{}", f, l),
            _ => String::new(),
        };
        let target = record.target();
        let level = record.level();
        let timestamp = Utc::now().format(TIMESTAMP_FORMAT);
        let message = format!(
            "{} {} [{}] {}: {}\n",
            timestamp,
            file_line,
            target,
            level,
            record.args()
        );
        if let Err(e) = self.write_message(&message) {
            eprintln!("{}", e);
        }
    }
    fn flush(&self) {}
}

pub fn builder() -> Builder<NoFilePath, NoMaxSize, NoMinSize, NoRotationRemove> {
    let file_path = NoFilePath;
    let max_size = NoMaxSize;
    let min_size = NoMinSize;
    let rotation_remove = NoRotationRemove;
    let builder = Builder {
        log_level: LogLevel::Trace,
        file_path,
        rotation: Rotation::Never,
        max_size,
        min_size,
        compress: false,
        delay_compress: false,
        if_empty: false,
        rotation_remove,
    };
    builder
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_logger() {
        let r = builder().file_path("output.log").rotation_count(1).finish();
        for i in (0..100) {
            crate::info!("message no: {}", i);
        }
        assert_eq!(r.is_ok(), true);
    }
}
