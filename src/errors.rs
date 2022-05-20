mod anyhow;
mod context;
mod generic_error;

pub use context::Context;
pub use generic_error::{GenericError, GenericResult};

use self::anyhow::GenericAnyhowedError;
