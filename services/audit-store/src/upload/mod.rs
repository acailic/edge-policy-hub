pub mod error;
pub mod queue;

pub use error::UploadError;
pub use queue::UploadQueue;

pub const DEFAULT_BATCH_SIZE: usize = 1_000;
pub const DEFAULT_UPLOAD_INTERVAL_SECS: u64 = 300;
