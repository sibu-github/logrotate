use logrotate::builder;
use logrotate::info;

#[test]
fn test_logger() {
    let r = builder().file_path("logs/output.log").finish();
    for i in 0..100 {
        info!("message no: {}", i);
    }
    assert_eq!(r.is_ok(), true);
}
