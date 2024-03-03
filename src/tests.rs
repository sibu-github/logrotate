use std::{
    fs::{read_dir, File},
    io::{Read, Write},
    path::Path,
};

use chrono::Utc;

use crate::utils::*;

mod test_utils {
    use std::{
        fs::{self, File},
        path::Path,
    };

    #[derive(Debug)]
    pub struct TestDataDir<'a>(&'a str);

    #[derive(Debug)]
    pub struct TestDataFile<'a>(&'a str);

    impl<'a> TestDataDir<'a> {
        pub fn create(p: &'a str) -> Self {
            let path = Path::new(p);
            if path.exists() {
                if !path.is_dir() {
                    panic!("path should be a directory");
                }
            } else {
                fs::create_dir_all(path).unwrap();
            }
            Self(p)
        }
    }

    impl<'a> Drop for TestDataDir<'a> {
        fn drop(&mut self) {
            let path = Path::new(self.0);
            if path.exists() {
                fs::remove_dir_all(path).unwrap();
            }
        }
    }

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
            let path = Path::new(self.0);
            if path.exists() {
                fs::remove_file(path).unwrap();
            }
        }
    }
}

#[test]
fn test_logger() {
    let file_name = "testloggeroutput.log";
    let r = crate::builder().file_path(file_name).finish();
    assert_eq!(r.is_ok(), true);
    for i in 0..100 {
        crate::info!("message no: {}", i);
    }
    let mut content = String::new();
    let mut file = File::open(file_name).unwrap();
    // let (_t, mut file) = test_utils::TestDataFile::create(file_name);
    file.read_to_string(&mut content).unwrap();
    eprintln!("{}", content);
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

#[test]
fn test_compress_old_files() {
    let r = compress_old_files("Cargo.toml", "Cargo", "lock");
    assert_eq!(r.is_err(), true);

    let dir_path = "compress_old_files_dir";
    let file_name = "output";
    let file_extn = "log";
    {
        let _test_data_dir = test_utils::TestDataDir::create(dir_path);
        let paths = vec![
            format!("{}/{}.{}", dir_path, file_name, file_extn),
            format!("{}/{}1.{}", dir_path, file_name, file_extn),
            format!("{}/{}2.{}", dir_path, file_name, file_extn),
            format!("{}/{}3.txt", dir_path, file_name),
        ];
        let inner_dir_path = format!("{}/inner_dir", dir_path);
        let _data_dir_2 = test_utils::TestDataDir::create(&inner_dir_path);
        let mut file_refs = vec![];
        for p in paths.iter() {
            let (rf, mut file) = test_utils::TestDataFile::create(p);
            file.write_all("some test data\n".as_bytes()).unwrap();
            file.flush().unwrap();
            file_refs.push(rf);
        }
        compress_old_files(dir_path, file_name, file_extn).unwrap();
        let files = read_dir(dir_path)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
            .collect::<Vec<_>>();
        assert_eq!(files.contains(&format!("output.log")), true);
        assert_eq!(files.contains(&format!("output.log.gz")), false);
        assert_eq!(files.contains(&format!("output1.log")), false);
        assert_eq!(files.contains(&format!("output1.log.gz")), true);
        assert_eq!(files.contains(&format!("output2.log")), false);
        assert_eq!(files.contains(&format!("output2.log.gz")), true);
        assert_eq!(files.contains(&format!("output3.txt")), true);
    }
    {
        let _test_data_dir = test_utils::TestDataDir::create("tstdir2");
        let paths = vec![
            format!("tstdir2/processtestlog"),
            format!("tstdir2/processtestlog1"),
            format!("tstdir2/processtestlog2"),
            format!("tstdir2/someothertstfile.txt"),
        ];
        let mut file_refs = vec![];
        for p in paths.iter() {
            let (rf, mut file) = test_utils::TestDataFile::create(p);
            file.write_all("some test data\n".as_bytes()).unwrap();
            file.flush().unwrap();
            file_refs.push(rf);
        }
        compress_old_files("tstdir2", "processtestlog", "").unwrap();
        let files = read_dir("tstdir2")
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
            .collect::<Vec<_>>();
        assert_eq!(files.contains(&format!("processtestlog")), true);
        assert_eq!(files.contains(&format!("processtestlog.gz")), false);
        assert_eq!(files.contains(&format!("processtestlog1")), false);
        assert_eq!(files.contains(&format!("processtestlog1.gz")), true);
        assert_eq!(files.contains(&format!("processtestlog2")), false);
        assert_eq!(files.contains(&format!("processtestlog2.gz")), true);
        assert_eq!(files.contains(&format!("someothertstfile.txt")), true);
    }
}

#[test]
fn test_remove_file_by_count() {
    let r = remove_file_by_count("Cargo.toml", "Cargo", "lock", 1);
    assert_eq!(r.is_err(), true);
    {
        let dir_path = "remove_file_dir";
        let file_name = "output";
        let file_extn = "log";
        let _test_data_dir = test_utils::TestDataDir::create(dir_path);
        let inner_dir_path = format!("{}/inner_dir", dir_path);
        let _data_dir_2 = test_utils::TestDataDir::create(&inner_dir_path);
        let paths = vec![
            format!("{}/{}.{}", dir_path, file_name, file_extn),
            format!("{}/{}1.{}", dir_path, file_name, file_extn),
            format!("{}/{}2.{}", dir_path, file_name, file_extn),
            format!("{}/{}3.txt", dir_path, file_name),
        ];
        let mut file_refs = vec![];
        for p in paths.iter() {
            let (rf, mut file) = test_utils::TestDataFile::create(p);
            file.write_all("some test data\n".as_bytes()).unwrap();
            file.flush().unwrap();
            file_refs.push(rf);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        remove_file_by_count(dir_path, file_name, file_extn, 1).unwrap();
        let files = read_dir(dir_path)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
            .collect::<Vec<_>>();
        assert_eq!(files.len(), 4);
        assert_eq!(files.contains(&format!("output.log")), true);
        assert_eq!(files.contains(&format!("output1.log")), false);
        assert_eq!(files.contains(&format!("output2.log")), true);
        assert_eq!(files.contains(&format!("output3.txt")), true);
    }
    {
        let dir_path = "remove_file_dir2";
        let file_name = "output";
        let file_extn = "";
        let _test_data_dir = test_utils::TestDataDir::create(dir_path);
        let paths = vec![
            format!("{}/{}", dir_path, file_name),
            format!("{}/{}1", dir_path, file_name),
            format!("{}/{}2", dir_path, file_name),
            format!("{}/{}3.txt", dir_path, file_name),
        ];
        let mut file_refs = vec![];
        for p in paths.iter() {
            let (rf, mut file) = test_utils::TestDataFile::create(p);
            file.write_all("some test data\n".as_bytes()).unwrap();
            file.flush().unwrap();
            file_refs.push(rf);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        remove_file_by_count(dir_path, file_name, file_extn, 1).unwrap();
        let files = read_dir(dir_path)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
            .collect::<Vec<_>>();
        assert_eq!(files.len(), 3);
        assert_eq!(files.contains(&format!("output")), true);
        assert_eq!(files.contains(&format!("output1")), false);
        assert_eq!(files.contains(&format!("output2")), true);
        assert_eq!(files.contains(&format!("output3.txt")), true);
    }
    {
        let dir_path = "remove_file_dir3";
        let file_name = "output";
        let file_extn = "";
        let _test_data_dir = test_utils::TestDataDir::create(dir_path);
        remove_file_by_count(dir_path, file_name, file_extn, 1).unwrap();
    }
}

#[test]
fn test_remove_files_by_age() {
    let dir_path = "remove_file_age_dir";
    let file_name = "output";
    let file_extn = "log";
    let inner_dir_path = format!("{}/inner_dir", dir_path);
    let _test_data_dir = test_utils::TestDataDir::create(dir_path);
    let _data_dir_2 = test_utils::TestDataDir::create(&inner_dir_path);
    let paths = vec![
        format!("{}/{}.{}", dir_path, file_name, file_extn),
        format!("{}/{}1.{}", dir_path, file_name, file_extn),
        format!("{}/{}2.{}", dir_path, file_name, file_extn),
        format!("{}/{}3.txt", dir_path, file_name),
    ];
    let mut file_refs = vec![];
    for p in paths.iter() {
        let (rf, mut file) = test_utils::TestDataFile::create(p);
        file.write_all("some test data\n".as_bytes()).unwrap();
        file.flush().unwrap();
        file_refs.push(rf);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    remove_files_by_age(dir_path, file_name, file_extn, 1).unwrap();
    let files = read_dir(dir_path)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert_eq!(files.len(), 5);
    assert_eq!(files.contains(&format!("output.log")), true);
    assert_eq!(files.contains(&format!("output1.log")), true);
    assert_eq!(files.contains(&format!("output2.log")), true);
    assert_eq!(files.contains(&format!("output3.txt")), true);
}
