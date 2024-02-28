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
pub(crate) struct Logger {
    pub(crate) log_level: LogLevel,
    pub(crate) log_dir: String,
    pub(crate) log_file_name: String,
    pub(crate) log_file_extn: String,
    pub(crate) file_handle: Mutex<File>,
    pub(crate) rotation_policy: RotationPolicy,
    pub(crate) next_rotation_time: RwLock<i64>,
    pub(crate) compress: bool,
    pub(crate) delay_compress: bool,
    pub(crate) if_empty: bool,
    pub(crate) rotation_remove: RotationRemove,
}

impl Logger {
    fn write_message(&self, message: &str) -> Result<(), Box<dyn Error + '_>> {
        let mut handle = self.file_handle.lock()?;
        handle.write_all(message.as_bytes())?;
        handle.flush()?;
        Ok(())
    }
    // TODO: Instead of reading file size from the metadata
    // keep a couter field which will keep on updating with write messages
    fn file_size(&self) -> Result<u64, Box<dyn Error + '_>> {
        let len = self.file_handle.lock()?.metadata()?.len();
        Ok(len)
    }

    fn rotate_log(&self) -> Result<(), Box<dyn Error + '_>> {
        if self.should_rotate()? {
            let mut file = self.file_handle.lock()?;
            let file = file.by_ref();
            let compress = self.compress && !self.delay_compress;
            let roll_path = rolled_log_path(
                &self.log_dir,
                &self.log_file_name,
                &self.log_file_extn,
                compress,
            );
            let out_file = File::create(&roll_path)?;
            copy_file(file, out_file, compress)?;
            truncate_file(file)?;
            self.remove_old_files()?;
            compress_old_files(&self.log_dir, &self.log_file_name, &self.log_file_extn)?;
        }
        Ok(())
    }

    fn remove_old_files(&self) -> Result<(), Box<dyn Error + '_>> {
        match self.rotation_remove {
            RotationRemove::ByCount(count) => remove_file_by_count(
                &self.log_dir,
                &self.log_file_name,
                &self.log_file_extn,
                count as usize,
            )?,
            RotationRemove::ByMaxAge(age) => {
                remove_files_by_age(&self.log_dir, &self.log_file_name, &self.log_file_extn, age)?
            }
        };
        Ok(())
    }

    fn compress_old_files(&self) -> Result<(), Box<dyn Error + '_>> {
        if !self.compress || !self.delay_compress {
            return Ok(());
        }

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
