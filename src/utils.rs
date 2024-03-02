use std::{
    any::Any,
    fs::{self, File, ReadDir},
    io::{self, Seek, SeekFrom},
    path::{Path, PathBuf},
    time::SystemTime,
};

use chrono::Utc;
use flate2::{write::GzEncoder, Compression};

pub(crate) type Size = u64;
pub(crate) type FileAge = u32;

pub(crate) const FL_NM_FORMAT: &str = "%Y-%m-%d-%T";
pub(crate) const TIMESTAMP_FORMAT: &str = "%Y-%m-%dT%T%.3f";
pub(crate) const MIN_AS_MILLI_SEC: i64 = 60 * 1000;
pub(crate) const HOUR_AS_MILLI_SEC: i64 = 60 * MIN_AS_MILLI_SEC;
pub(crate) const DAY_AS_MILLI_SEC: i64 = 24 * HOUR_AS_MILLI_SEC;
pub(crate) const WEEK_AS_MILLI_SEC: i64 = 7 * DAY_AS_MILLI_SEC;
pub(crate) const MONTH_AS_MILLI_SEC: i64 = 30 * DAY_AS_MILLI_SEC;
pub(crate) const YEAR_AS_MILLI_SEC: i64 = 365 * DAY_AS_MILLI_SEC;

pub(crate) fn get_size(s: &dyn Any) -> Option<Size> {
    s.downcast_ref::<Size>().cloned()
}

// check if the current timestamp has crossed next_rotation_time
pub(crate) fn has_crossed_rotation_time(next_rotation_time: i64) -> bool {
    let curr_ts = Utc::now().timestamp_millis();
    next_rotation_time > 0 && next_rotation_time <= curr_ts
}

pub(crate) fn log_file_path(log_dir: &str, log_file_name: &str, log_file_extn: &str) -> PathBuf {
    let mut path = PathBuf::new();
    if !log_dir.is_empty() {
        path = PathBuf::from(log_dir);
    }
    assert!(!log_file_name.is_empty());
    let file_name = if log_file_extn.is_empty() {
        format!("{}", log_file_name)
    } else {
        format!("{}.{}", log_file_name, log_file_extn)
    };
    path.push(file_name);
    path
}

pub(crate) fn rolled_log_path(
    log_dir: &str,
    log_file_name: &str,
    log_file_extn: &str,
    compress: bool,
) -> PathBuf {
    let ts = Utc::now().format(FL_NM_FORMAT);
    let mut path = PathBuf::new();
    if !log_dir.is_empty() {
        path = PathBuf::from(log_dir);
    }
    assert!(!log_file_name.is_empty());
    let mut file_name = if log_file_extn.is_empty() {
        format!("{}.{}", log_file_name, ts)
    } else {
        format!("{}.{}.{}", log_file_name, ts, log_file_extn)
    };
    if compress {
        file_name = format!("{}.gz", file_name);
    }
    path.push(file_name);
    path
}

// split the given path into parent directory, file name and file extension
pub(crate) fn split_file_path(path: &Path) -> (String, String, String) {
    let parent_dir = path
        .parent()
        .map(|v| v.display().to_string())
        .unwrap_or_default();
    let mut file_name = path
        .file_name()
        .map(|v| v.to_string_lossy().to_string())
        .unwrap_or_default();
    let extn = path
        .extension()
        .map(|v| v.to_string_lossy().to_string())
        .unwrap_or_default();
    if !extn.is_empty() {
        file_name = file_name
            .strip_suffix(&extn)
            .unwrap_or_default()
            .strip_suffix(".")
            .unwrap_or_default()
            .to_string();
    }
    (parent_dir, file_name, extn)
}

// truncate the file and delete all content
pub(crate) fn truncate_file(file: &mut File) -> std::io::Result<()> {
    file.set_len(0)?;
    file.seek(SeekFrom::Start(0))?;
    Ok(())
}

pub(crate) fn copy_file(src: &mut File, mut dst: File, compress: bool) -> io::Result<()> {
    if compress {
        let mut encoder = GzEncoder::new(dst, Compression::default());
        io::copy(src, &mut encoder)?;
        encoder.finish()?;
    } else {
        io::copy(src, &mut dst).map_err(|err| {
            eprintln!("{:?}", err);
            err
        })?;
    }

    Ok(())
}

pub(crate) fn max_age(age: FileAge) -> u64 {
    age as u64 * 24 * 3600
}

pub(crate) fn duration_since(time: SystemTime) -> u64 {
    let now = SystemTime::now();
    match now.duration_since(time) {
        Ok(n) => n.as_secs(),
        Err(e) => {
            eprintln!("{}", e);
            0
        }
    }
}

pub(crate) fn get_file_age(path: &Path) -> io::Result<u64> {
    if !path.is_file() {
        return Ok(0);
    }
    let file = File::open(path)?;
    let created_time = file.metadata()?.created()?;
    let duration = duration_since(created_time);
    Ok(duration)
}

pub(crate) fn remove_files_by_age(
    dir: &str,
    file_name: &str,
    file_extn: &str,
    age: FileAge,
) -> io::Result<()> {
    let curr_file = format!("{}.{}", file_name, file_extn);
    for entry in read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let (name, extn) = file_name_and_extension(&path);
        if name.starts_with(file_name)
            && !name.eq(&curr_file)
            && (extn.eq(file_extn) || extn.eq("gz"))
            && get_file_age(&path)? > max_age(age)
        {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

pub(crate) fn remove_file_by_count(
    dir: &str,
    file_name: &str,
    file_extn: &str,
    count: usize,
) -> io::Result<()> {
    let curr_file = format!("{}.{}", file_name, file_extn);
    let mut entries = vec![];
    for entry in read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let (name, extn) = file_name_and_extension(&path);
            if name.starts_with(file_name)
                && !name.eq(&curr_file)
                && (extn.eq(file_extn) || extn.eq("gz"))
            {
                let created_time = path.metadata()?.created()?;
                entries.push((entry, created_time));
            }
        }
    }
    if entries.len() == 0 || entries.len() < count {
        return Ok(());
    }
    entries.sort_unstable_by(|a, b| a.1.cmp(&b.1));
    for _ in 0..count {
        entries.pop();
    }
    for (entry, _) in entries {
        fs::remove_file(entry.path())?;
    }
    Ok(())
}

pub(crate) fn compress_old_files<'a>(
    dir: &'a str,
    file_name: &'a str,
    file_extn: &'a str,
) -> Result<(), Box<dyn std::error::Error + 'a>> {
    let curr_file = format!("{}.{}", file_name, file_extn);
    for entry in read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let (name, extn) = file_name_and_extension(&path);
        if name.starts_with(file_name) && !name.eq(&curr_file) && extn.eq(file_extn) {
            {
                let compress_file = format!("{}.gz", name);
                let mut p = path.to_path_buf();
                p.pop();
                p.push(compress_file);
                let mut src = File::open(&path)?;
                let dst = File::create(p)?;
                let mut encoder = GzEncoder::new(dst, Compression::default());
                io::copy(&mut src, &mut encoder)?;
                encoder.finish()?;
            }
            fs::remove_file(&path)?;
        }
    }
    Ok(())
}

fn dir_path(dir: &str) -> io::Result<&Path> {
    let dir = if dir.is_empty() { "." } else { dir };
    let dir = Path::new(dir);
    if !dir.is_dir() {
        return Err(io::Error::other("log_dir not a valid directory path"));
    }
    Ok(dir)
}

fn read_dir(dir: &str) -> io::Result<ReadDir> {
    let dir = dir_path(dir)?;
    fs::read_dir(dir)
}

fn file_name_and_extension(path: &Path) -> (&str, &str) {
    let name = path.file_name().unwrap_or_default();
    let name = name.to_str().unwrap_or_default();
    let extn = path.extension().unwrap_or_default();
    let extn = extn.to_str().unwrap_or_default();
    (name, extn)
}
