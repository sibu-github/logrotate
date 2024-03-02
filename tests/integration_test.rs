use logrotate::builder;
use logrotate::info;

#[test]
#[ignore]
fn test_logger() {
    let r = builder()
        .file_path("logs/output.log")
        .max_size(2 * 1024)
        .rotation_count(5)
        .compress(true)
        .delay_compress(true)
        .finish();
    for i in 0..400 {
        info!("message no: {}", i);
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    assert_eq!(r.is_ok(), true);
}
