pub mod error;
pub mod signer;

pub use error::SigningError;
pub use signer::Signer;

pub const SIGNATURE_ALGORITHM: &str = "HMAC-SHA256";
pub const SIGNATURE_VERSION: u8 = 1;
