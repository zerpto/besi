//! Support for long-running commands (`api`, `cron`): stderr logging and
//! the shutdown signal.

mod logging;
mod shutdown;

pub(crate) use logging::init as init_logging;
pub(crate) use shutdown::signal as shutdown_signal;
