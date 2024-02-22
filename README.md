# logrotate

A simple logger that prints all messages with a readable output format and with file rotate functionality. The file rotation works similar to Linux's `logrotate` utility.


## Usage
```
use logrotate::Logger;

fn main() {
    Logger::builder().default().finish();
    log::info!("Some log messages");
}
```