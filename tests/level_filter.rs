use std::fs;

use logrotate::builder;
use logrotate::{debug, error, info};

#[test]
fn test_logger_level_filter() {
    let path = "logs/levelfilter.log";
    fs::File::create(path).unwrap();
    let r = builder()
        .log_level(log::Level::Info)
        .file_path(path)
        .max_size(2 * 1024)
        .rotation_count(0)
        .finish();
    assert_eq!(r.is_ok(), true);
    debug!("Some message on Debug level");
    info!("Some message on Info level");
    error!("Some message on Error level");
    let content = fs::read_to_string(path).unwrap();
    let lines = content.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 2);
}
