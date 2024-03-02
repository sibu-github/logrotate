use builder::*;

pub use log::debug;
pub use log::error;
pub use log::info;
pub use log::trace;
pub use log::warn;

mod builder;
mod logger;
mod utils;

#[cfg(test)]
mod tests;

pub fn builder() -> Builder<NoFilePath, NoMaxSize, NoMinSize> {
    let file_path = NoFilePath;
    let max_size = NoMaxSize;
    let min_size = NoMinSize;
    let rotation_remove = RotationRemove::ByCount(0);
    let builder = Builder {
        log_level: log::Level::Trace,
        file_path,
        rotation_time: RotationTime::Never,
        max_size,
        min_size,
        compress: false,
        delay_compress: false,
        rotation_remove,
    };
    builder
}
