use logrotate::builder;
use logrotate::info;

#[test]
fn test_logger_max_age() {
    let r = builder()
        .log_level(log::Level::Info)
        .file_path("logs/max-age.log")
        .max_age(3)
        .finish();
    for i in 0..10 {
        info!("message no: {}", i);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert_eq!(r.is_ok(), true);
}
