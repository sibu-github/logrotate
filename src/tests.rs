use std::{fs::File, io::Write, path::Path};

use chrono::Utc;

use crate::{builder, utils::*};

mod test_utils {
    use std::fs::{self, File};

    #[derive(Debug)]
    pub struct TestDataFile<'a>(&'a str);

    impl<'a> TestDataFile<'a> {
        pub fn create(path: &'a str) -> (Self, File) {
            let file = File::options()
                .append(true)
                .create(true)
                .open(path)
                .unwrap();
            (Self(path), file)
        }
    }

    impl<'a> Drop for TestDataFile<'a> {
        fn drop(&mut self) {
            fs::remove_file(self.0).unwrap();
        }
    }
}

#[test]
fn test_logger() {
    let r = builder().file_path("output.log").finish();
    for i in 0..100 {
        crate::info!("message no: {}", i);
    }
    assert_eq!(r.is_ok(), true);
}
#[test]
fn test_get_size() {
    let val = 10 as Size;
    assert_eq!(get_size(&val), Some(val));
    let val = "abcd";
    assert_eq!(get_size(&val), None);
    let val = 1_i32;
    assert_eq!(get_size(&val), None);
}

#[test]
fn test_has_crossed_rotation_time() {
    let next_rotation_time = Utc::now().timestamp_millis() + 5;
    assert_eq!(has_crossed_rotation_time(next_rotation_time), false);
    let next_rotation_time = Utc::now().timestamp_millis();
    assert_eq!(has_crossed_rotation_time(next_rotation_time), true);
    let next_rotation_time = Utc::now().timestamp_millis() - 1;
    assert_eq!(has_crossed_rotation_time(next_rotation_time), true);
}

#[test]
fn test_log_file_path() {
    let path = log_file_path("", "output", "log");
    assert_eq!(path.display().to_string(), "output.log");
    let path = log_file_path("logs", "output", "log");
    assert_eq!(path.display().to_string(), "logs/output.log");
    let path = log_file_path(".", "output", "log");
    assert_eq!(path.display().to_string(), "./output.log");
    let path = log_file_path("", "output", "");
    assert_eq!(path.display().to_string(), "output");
    let path = log_file_path("logs", "output", "");
    assert_eq!(path.display().to_string(), "logs/output");
}

#[test]
fn test_rolled_log_path() {
    let path = rolled_log_path("", "output", "log", false);
    let file_name = path.file_name().unwrap().to_string_lossy().to_string();
    assert_eq!(file_name.starts_with("output"), true);
    let extn = path.extension().unwrap().to_string_lossy().to_string();
    assert_eq!(extn, "log");
    let path = rolled_log_path("logs", "output", "log", false);
    assert_eq!(path.display().to_string().starts_with("logs"), true);
    let file_name = path.file_name().unwrap().to_string_lossy().to_string();
    assert_eq!(file_name.starts_with("output"), true);
    let extn = path.extension().unwrap().to_string_lossy().to_string();
    assert_eq!(extn, "log");
    let path = rolled_log_path("logs", "output", "log", true);
    assert_eq!(path.display().to_string().starts_with("logs"), true);
    let file_name = path.file_name().unwrap().to_string_lossy().to_string();
    assert_eq!(file_name.starts_with("output"), true);
    let extn = path.extension().unwrap().to_string_lossy().to_string();
    assert_eq!(extn, "gz");
    let path = rolled_log_path("logs", "output", "", true);
    assert_eq!(path.display().to_string().starts_with("logs"), true);
    let file_name = path.file_name().unwrap().to_string_lossy().to_string();
    assert_eq!(file_name.starts_with("output"), true);
    let extn = path.extension().unwrap().to_string_lossy().to_string();
    assert_eq!(extn, "gz");
}

#[test]
fn test_split_file_path() {
    let path = Path::new("./output.log");
    let (dir, file_name, extn) = split_file_path(path);
    assert_eq!(dir, ".");
    assert_eq!(file_name, "output");
    assert_eq!(extn, "log");

    let path = Path::new("/var/log/output.log");
    let (dir, file_name, extn) = split_file_path(path);
    assert_eq!(dir, "/var/log");
    assert_eq!(file_name, "output");
    assert_eq!(extn, "log");

    let path = Path::new("/var/log/output.2024-03-02-03:22:36.log.gz");
    let (dir, file_name, extn) = split_file_path(path);
    assert_eq!(dir, "/var/log");
    assert_eq!(file_name, "output.2024-03-02-03:22:36.log");
    assert_eq!(extn, "gz");

    let path = Path::new("/var/log/output");
    let (dir, file_name, extn) = split_file_path(path);
    assert_eq!(dir, "/var/log");
    assert_eq!(file_name, "output");
    assert_eq!(extn, "");
}

#[test]
fn test_truncate_file() {
    let path = "truncate_file_test.txt";
    let (_test_data_file, mut file) = test_utils::TestDataFile::create(path);
    file.write_all("some dummy text\n".as_bytes()).unwrap();
    let size = file.metadata().unwrap().len();
    assert_eq!(size > 0, true);
    truncate_file(&mut file).unwrap();
    let size = file.metadata().unwrap().len();
    assert_eq!(size, 0);
}

#[test]
fn test_copy_file() {
    {
        let path = "Cargo_copied.toml";
        let (_test_data_file, file) = test_utils::TestDataFile::create(path);
        let mut src = File::open("Cargo.toml").unwrap();
        copy_file(&mut src, file, false).unwrap();
        assert_eq!(Path::new(path).exists(), true);
        let file = File::open(path).unwrap();
        assert_eq!(
            file.metadata().unwrap().len(),
            src.metadata().unwrap().len()
        );
    }
    {
        let path = "Cargo_copied.toml.gz";
        let (_test_data_file, file) = test_utils::TestDataFile::create(path);
        let mut src = File::open("Cargo.toml").unwrap();
        copy_file(&mut src, file, true).unwrap();
        assert_eq!(Path::new(path).exists(), true);
        let file = File::open(path).unwrap();
        assert_eq!(
            file.metadata().unwrap().len() > 0
                && file.metadata().unwrap().len() <= src.metadata().unwrap().len(),
            true
        );
    }
}

#[test]
fn test_max_age() {
    assert_eq!(max_age(1), 24 * 3600);
    assert_eq!(max_age(2), 2 * 24 * 3600);
}

#[test]
fn test_get_file_age() {
    let path = Path::new("src");
    assert_eq!(get_file_age(path).unwrap(), 0);
    let path = Path::new("Cargo.toml");
    assert_eq!(get_file_age(path).unwrap() > 0, true);
}
