/// Main (init) socket nanomsg URL
pub(crate) const MAIN_SOCKET_URL: &str = "ipc:///tmp/nanoserver-main.ipc";

/// Until constexpr are stabilized, we have to resort to macros
macro_rules! CLIENT_SOCKET_URL {
    ($socket_id: expr) => {
        format!("ipc:///tmp/nanoserver-client-{}.ipc", $socket_id)
    };
}

pub(crate) const DEFAULT_SESSION_TIMEOUT: u64 = 30;
pub(crate) const DEFAULT_GC_INTERVAL: u64 = 60;
