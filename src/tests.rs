use crate::builder;

#[test]
fn test_logger() {
    let r = builder().file_path("output.log").finish();
    for i in 0..100 {
        crate::info!("message no: {}", i);
    }
    assert_eq!(r.is_ok(), true);
}
