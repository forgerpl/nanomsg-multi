use consts::{DEFAULT_GC_INTERVAL, DEFAULT_SESSION_TIMEOUT, MAIN_SOCKET_URL};

use std::time::{Duration, Instant};

#[inline]
fn client_socket(socket_id: usize) -> String {
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

impl Into<Duration> for SessionTimeout {
    fn into(self) -> Duration {
        self.0
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

impl Into<Duration> for GcInterval {
    fn into(self) -> Duration {
        self.0
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
