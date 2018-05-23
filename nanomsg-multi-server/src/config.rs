use consts::{DEFAULT_GC_INTERVAL, DEFAULT_SESSION_TIMEOUT, MAIN_SOCKET_URL};

use std::time::Duration;

pub type Uid = ::libc::uid_t;
pub type Gid = ::libc::gid_t;
pub type FileMode = u32;

#[inline]
pub fn default_client_socket_url(socket_id: usize) -> String {
    CLIENT_SOCKET_URL!(socket_id)
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SessionTimeout(Duration);

impl Default for SessionTimeout {
    fn default() -> SessionTimeout {
        SessionTimeout(Duration::from_secs(DEFAULT_SESSION_TIMEOUT))
    }
}

impl From<Duration> for SessionTimeout {
    fn from(source: Duration) -> SessionTimeout {
        SessionTimeout(source)
    }
}

impl From<SessionTimeout> for Duration {
    fn from(source: SessionTimeout) -> Duration {
        source.0
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct GcInterval(Duration);

impl Default for GcInterval {
    fn default() -> GcInterval {
        GcInterval(Duration::from_secs(DEFAULT_GC_INTERVAL))
    }
}

impl From<Duration> for GcInterval {
    fn from(source: Duration) -> GcInterval {
        GcInterval(source)
    }
}

impl From<GcInterval> for Duration {
    fn from(source: GcInterval) -> Duration {
        source.0
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct MainSocketUrl<'s>(&'s str);

impl<'s> Default for MainSocketUrl<'s> {
    fn default() -> MainSocketUrl<'s> {
        MainSocketUrl(MAIN_SOCKET_URL)
    }
}

impl<'s> From<&'s str> for MainSocketUrl<'s> {
    fn from(source: &'s str) -> MainSocketUrl<'s> {
        MainSocketUrl(source)
    }
}

impl<'s> AsRef<str> for MainSocketUrl<'s> {
    fn as_ref(&self) -> &str {
        self.0
    }
}
