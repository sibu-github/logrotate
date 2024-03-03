use std::{
    error::Error,
    fs::File,
    io::{self, Write},
    path::PathBuf,
    sync::{Mutex, RwLock},
};

use chrono::Utc;
use log::Level as LogLevel;

use crate::{
    builder::{RotationPolicy, RotationRemove},
    utils::*,
};

#[derive(Debug)]
pub(crate) struct FileHandle {
    inner: File,
    size: u64,
    dir: String,
    file_name: String,
    file_extn: String,
}

#[derive(Debug)]
pub(crate) struct Logger {
    pub(crate) log_level: LogLevel,
    pub(crate) file_handle: Mutex<FileHandle>,
    pub(crate) rotation_policy: RotationPolicy,
    pub(crate) next_rotation_time: RwLock<i64>,
    pub(crate) compress: bool,
    pub(crate) delay_compress: bool,
    pub(crate) rotation_remove: RotationRemove,
}

impl FileHandle {
    pub(crate) fn new(
        inner: File,
        size: u64,
        dir: String,
        file_name: String,
        file_extn: String,
    ) -> Self {
        Self {
            inner,
            size,
            dir,
            file_name,
            file_extn,
        }
    }

    pub(crate) fn log_path(&self) -> PathBuf {
        log_file_path(&self.dir, &self.file_name, &self.file_extn)
    }

    pub(crate) fn rolled_log_path(&self, compress: bool) -> PathBuf {
        rolled_log_path(&self.dir, &self.file_name, &self.file_extn, compress)
    }

    pub(crate) fn write_message(&mut self, message: &str) -> io::Result<()> {
        let size = message.len() as u64;
        let file = self.inner.by_ref();
        file.write_all(message.as_bytes())?;
        file.flush()?;
        self.size += size;
        Ok(())
    }

    pub(crate) fn truncate(&mut self) -> io::Result<()> {
        truncate_file(&mut self.inner)?;
        self.size = 0;
        Ok(())
    }

    pub(crate) fn rollover(&mut self, compress: bool) -> io::Result<()> {
        let roll_path = self.rolled_log_path(compress);
        let log_path = self.log_path();
        let out_file = File::create(&roll_path)?;
        let mut file = File::open(&log_path)?;
        copy_file(&mut file, out_file, compress)?;
        Ok(())
    }

    pub(crate) fn compress_old_files(&self) -> io::Result<()> {
        compress_old_files(&self.dir, &self.file_name, &self.file_extn)?;
        Ok(())
    }

    pub(crate) fn remove_file_by_count(&self, count: usize) -> io::Result<()> {
        remove_file_by_count(&self.dir, &self.file_name, &self.file_extn, count)
    }

    pub(crate) fn remove_files_by_age(&self, age: FileAge) -> io::Result<()> {
        remove_files_by_age(&self.dir, &self.file_name, &self.file_extn, age)
    }
}

impl Logger {
    fn write_message(&self, message: &str) -> Result<(), Box<dyn Error + '_>> {
        self.rotate_log()?;
        let mut handle = self.file_handle.lock()?;
        handle.write_message(message)?;
        Ok(())
    }

    fn file_size(&self) -> Result<u64, Box<dyn Error + '_>> {
        Ok(self.file_handle.lock()?.size)
    }

    fn rotate_log(&self) -> Result<(), Box<dyn Error + '_>> {
        if !self.should_rotate()? {
            return Ok(());
        }
        let mut handle = self.file_handle.lock()?;
        match self.rotation_remove {
            RotationRemove::ByCount(count) => {
                let count = if count > 0 { count as usize - 1 } else { 0 };
                handle.remove_file_by_count(count)?;
            }
            RotationRemove::ByMaxAge(age) => {
                handle.remove_files_by_age(age)?;
            }
        };
        if self.compress && self.delay_compress {
            handle.compress_old_files()?;
        }
        if !self.is_zero_rotation_remove() {
            let compress = self.compress && !self.delay_compress;
            handle.rollover(compress)?;
        }
        handle.truncate()?;
        Ok(())
    }

    fn is_zero_rotation_remove(&self) -> bool {
        match self.rotation_remove {
            RotationRemove::ByCount(0) => true,
            _ => false,
        }
    }

    fn update_next_rotation_time(&self) -> Result<(), Box<dyn Error + '_>> {
        let next_rotation_time = self.rotation_policy.next_rotation_time();
        if next_rotation_time > 0 {
            // TODO: Use try_write instead
            let mut rotation = self.next_rotation_time.write()?;
            *rotation = next_rotation_time;
        }
        Ok(())
    }

    fn should_rotate(&self) -> Result<bool, Box<dyn Error + '_>> {
        let next_rotation_time = self.next_rotation_time.read()?.clone();
        let file_size = self.file_size()?;
        if has_crossed_rotation_time(next_rotation_time) {
            self.update_next_rotation_time()?;
        }
        let val = match self.rotation_policy {
            RotationPolicy::MaxSizeOnly(size) => file_size >= size,
            RotationPolicy::MaxSizeOrRotationTime(size, _) => {
                has_crossed_rotation_time(next_rotation_time) || file_size >= size
            }
            RotationPolicy::MinSizeAndRotationTime(size, _) => {
                has_crossed_rotation_time(next_rotation_time) && file_size >= size
            }
            RotationPolicy::RotationTimeOnly(_) => has_crossed_rotation_time(next_rotation_time),
        };
        Ok(val)
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
