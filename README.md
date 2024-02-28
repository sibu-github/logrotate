# logrotate

A simple logger that prints all messages with a readable output format and with file rotate functionality. The file rotation works similar to Linux's `logrotate` utility.

## Usage

```
fn main() {
    logrotate::builder().file_path("output.log").finish();
    log::info!("Some log messages");
}
```
