use std::{
    error::Error,
    fs::File,
    io::Write,
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
}

#[derive(Debug)]
pub(crate) struct Logger {
    pub(crate) log_level: LogLevel,
    pub(crate) log_dir: String,
    pub(crate) log_file_name: String,
    pub(crate) log_file_extn: String,
    pub(crate) file_handle: Mutex<Option<FileHandle>>,
    pub(crate) rotation_policy: RotationPolicy,
    pub(crate) next_rotation_time: RwLock<i64>,
    pub(crate) compress: bool,
    pub(crate) delay_compress: bool,
    pub(crate) rotation_remove: RotationRemove,
}

impl FileHandle {
    pub(crate) fn new(inner: File, size: u64) -> Self {
        Self { inner, size }
    }

    pub(crate) fn write_message(&mut self, message: &str) -> std::io::Result<()> {
        let size = message.len() as u64;
        let file = self.inner.by_ref();
        file.write_all(message.as_bytes())?;
        file.flush()?;
        self.size += size;
        Ok(())
    }
}

impl Logger {
    fn write_message(&self, message: &str) -> Result<(), Box<dyn Error + '_>> {
        self.rotate_log()?;
        let mut handle = self.file_handle.lock()?;
        if let Some(handle) = handle.as_mut() {
            handle.write_message(message)?;
        }
        Ok(())
    }

    fn file_size(&self) -> Result<u64, Box<dyn Error + '_>> {
        let file_handle = self.file_handle.lock()?;
        let val = match *file_handle {
            Some(ref handle) => handle.size,
            _ => 0,
        };
        Ok(val)
    }

    fn rotate_log(&self) -> Result<(), Box<dyn Error + '_>> {
        if !self.should_rotate()? {
            return Ok(());
        }
        self.remove_old_files()?;
        self.compress_old_files()?;
        // NOTE: lock on file_handle should be kept until file is truncated
        // and file_size reset
        let mut handle = self.file_handle.lock()?;
        let log_path = log_file_path(&self.log_dir, &self.log_file_name, &self.log_file_extn);
        if !self.is_zero_rotation_remove() {
            let compress = self.compress && !self.delay_compress;
            let roll_path = rolled_log_path(
                &self.log_dir,
                &self.log_file_name,
                &self.log_file_extn,
                compress,
            );
            let out_file = File::create(&roll_path)?;
            handle.take().ok_or("counld not get file handle")?;
            let mut file = File::open(&log_path)?;
            copy_file(&mut file, out_file, compress)?;
        }
        let mut file = File::options().append(true).create(true).open(&log_path)?;
        truncate_file(&mut file)?;
        *handle = Some(FileHandle::new(file, 0));
        Ok(())
    }

    fn is_zero_rotation_remove(&self) -> bool {
        match self.rotation_remove {
            RotationRemove::ByCount(0) => true,
            _ => false,
        }
    }

    fn remove_old_files(&self) -> Result<(), Box<dyn Error + '_>> {
        match self.rotation_remove {
            RotationRemove::ByCount(count) => {
                let count = if count > 0 { count as usize - 1 } else { 0 };
                remove_file_by_count(
                    &self.log_dir,
                    &self.log_file_name,
                    &self.log_file_extn,
                    count,
                )
            }
            RotationRemove::ByMaxAge(age) => {
                remove_files_by_age(&self.log_dir, &self.log_file_name, &self.log_file_extn, age)
            }
        }?;
        Ok(())
    }

    fn compress_old_files(&self) -> Result<(), Box<dyn Error + '_>> {
        if !self.compress || !self.delay_compress {
            return Ok(());
        }
        compress_old_files(&self.log_dir, &self.log_file_name, &self.log_file_extn)?;
        Ok(())
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
            RotationPolicy::MaxSizeOnly(size) => {
                eprintln!("file_size: {}, size: {}", file_size, size);
                file_size >= size
            }
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
