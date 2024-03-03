use std::{
    fs::{self, File},
    sync::{Mutex, RwLock},
};

use chrono::Utc;
use log::Level as LogLevel;

use crate::{
    logger::{FileHandle, Logger},
    utils::*,
};

#[derive(Debug, Clone, Copy)]
pub enum RotationTime {
    Minutely,
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Yearly,
    Never,
}

#[derive(Debug)]
pub enum RotationPolicy {
    MaxSizeOnly(Size),
    MaxSizeOrRotationTime(Size, RotationTime),
    MinSizeAndRotationTime(Size, RotationTime),
    RotationTimeOnly(RotationTime),
}

#[derive(Debug)]
pub enum RotationRemove {
    ByMaxAge(FileAge),
    ByCount(u32),
}

pub struct NoFilePath;
pub struct NoMaxSize;
pub struct NoMinSize;

#[derive(Debug)]
pub struct Builder<T, U, V> {
    pub(crate) log_level: LogLevel,
    pub(crate) file_path: T,
    pub(crate) rotation_time: RotationTime,
    pub(crate) max_size: U,
    pub(crate) min_size: V,
    pub(crate) compress: bool,
    pub(crate) delay_compress: bool,
    pub(crate) rotation_remove: RotationRemove,
}

impl RotationTime {
    pub(crate) fn next_rotation_time(&self) -> i64 {
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

impl RotationPolicy {
    pub(crate) fn next_rotation_time(&self) -> i64 {
        match self {
            Self::MaxSizeOrRotationTime(_, rt) => rt.next_rotation_time(),
            Self::MinSizeAndRotationTime(_, rt) => rt.next_rotation_time(),
            Self::RotationTimeOnly(rt) => rt.next_rotation_time(),
            _ => 0,
        }
    }
}

impl<T, U, V> Builder<T, U, V> {
    pub fn log_level(mut self, log_level: LogLevel) -> Self {
        self.log_level = log_level;
        self
    }
    pub fn minutely(mut self) -> Self {
        self.rotation_time = RotationTime::Minutely;
        self
    }
    pub fn hourly(mut self) -> Self {
        self.rotation_time = RotationTime::Hourly;
        self
    }
    pub fn daily(mut self) -> Self {
        self.rotation_time = RotationTime::Daily;
        self
    }
    pub fn weekly(mut self) -> Self {
        self.rotation_time = RotationTime::Weekly;
        self
    }
    pub fn monthly(mut self) -> Self {
        self.rotation_time = RotationTime::Monthly;
        self
    }
    pub fn yearly(mut self) -> Self {
        self.rotation_time = RotationTime::Yearly;
        self
    }
    pub fn compress(mut self, compress: bool) -> Self {
        self.compress = compress;
        self
    }
    pub fn delay_compress(mut self, delay_compress: bool) -> Self {
        self.delay_compress = delay_compress;
        self
    }
}

impl<T, U, V> Builder<T, U, V> {
    pub fn rotation_count(self, count: u32) -> Builder<T, U, V> {
        let rotation_remove = RotationRemove::ByCount(count);
        Builder {
            log_level: self.log_level,
            file_path: self.file_path,
            rotation_time: self.rotation_time,
            max_size: self.max_size,
            min_size: self.min_size,
            compress: self.compress,
            delay_compress: self.delay_compress,
            rotation_remove,
        }
    }
    pub fn max_age(self, age: FileAge) -> Builder<T, U, V> {
        let rotation_remove = RotationRemove::ByMaxAge(age);
        Builder {
            log_level: self.log_level,
            file_path: self.file_path,
            rotation_time: self.rotation_time,
            max_size: self.max_size,
            min_size: self.min_size,
            compress: self.compress,
            delay_compress: self.delay_compress,
            rotation_remove,
        }
    }
}

impl<T> Builder<T, NoMaxSize, NoMinSize> {
    pub fn max_size(self, max_size: Size) -> Builder<T, Size, NoMinSize> {
        Builder {
            log_level: self.log_level,
            file_path: self.file_path,
            rotation_time: self.rotation_time,
            max_size,
            min_size: self.min_size,
            compress: self.compress,
            delay_compress: self.delay_compress,
            rotation_remove: self.rotation_remove,
        }
    }
    pub fn min_size(self, min_size: Size) -> Builder<T, NoMaxSize, Size> {
        Builder {
            log_level: self.log_level,
            file_path: self.file_path,
            rotation_time: self.rotation_time,
            max_size: self.max_size,
            min_size,
            compress: self.compress,
            delay_compress: self.delay_compress,
            rotation_remove: self.rotation_remove,
        }
    }
}

impl<U, V> Builder<NoFilePath, U, V> {
    pub fn file_path(self, file_path: &str) -> Builder<String, U, V> {
        Builder {
            log_level: self.log_level,
            file_path: file_path.to_owned(),
            rotation_time: self.rotation_time,
            max_size: self.max_size,
            min_size: self.min_size,
            compress: self.compress,
            delay_compress: self.delay_compress,
            rotation_remove: self.rotation_remove,
        }
    }
}

impl<U: 'static, V: 'static> Builder<String, U, V> {
    pub fn rotation_policy(&self) -> RotationPolicy {
        let rotation_time = self.rotation_time;
        let max_size = get_size(&self.max_size);
        let min_size = get_size(&self.min_size);
        match (max_size, min_size, rotation_time) {
            (Some(max_size), _, RotationTime::Never) => RotationPolicy::MaxSizeOnly(max_size),
            (Some(max_size), _, _) => {
                RotationPolicy::MaxSizeOrRotationTime(max_size, rotation_time)
            }
            (_, Some(min_size), _) => {
                RotationPolicy::MinSizeAndRotationTime(min_size, rotation_time)
            }
            _ => RotationPolicy::RotationTimeOnly(rotation_time),
        }
    }

    pub fn finish(self) -> Result<(), Box<dyn std::error::Error>> {
        if self.file_path.is_empty() {
            return Err("file_path cannot be empty".into());
        }
        let file_path = std::path::Path::new(&self.file_path);
        let (dir, file_name, file_extn) = split_file_path(file_path);
        if file_name.is_empty() {
            return Err("log_file_name cannot be empty".into());
        }
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let next_rotation_time = RwLock::new(self.rotation_time.next_rotation_time());
        let file = File::options()
            .create(true)
            .append(true)
            .open(&self.file_path)?;
        let size = file.metadata()?.len();
        let file_handle = FileHandle::new(file, size, dir, file_name, file_extn);
        let file_handle = Mutex::new(file_handle);
        let logger = Logger {
            log_level: self.log_level,
            file_handle,
            rotation_policy: self.rotation_policy(),
            next_rotation_time,
            compress: self.compress,
            delay_compress: self.delay_compress,
            rotation_remove: self.rotation_remove,
        };
        log::set_max_level(self.log_level.to_level_filter());
        log::set_boxed_logger(Box::new(logger))?;
        Ok(())
    }
}
