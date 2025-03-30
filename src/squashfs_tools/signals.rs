use std::time::Duration;
use signal_hook::iterator::Signals;
use crate::error::{SquashError, Result};

#[cfg(target_os = "macos")]
use libc::{sigwait, sigset_t};

#[cfg(target_os = "openbsd")]
use libc::{sigwait, sigset_t};

#[cfg(not(any(target_os = "macos", target_os = "openbsd")))]
use libc::{sigtimedwait, sigwaitinfo, sigset_t, timespec};

pub struct SignalWaiter {
    signals: Signals,
    waiting: bool,
}

impl SignalWaiter {
    pub fn new(signals: &[i32]) -> Result<Self> {
        Ok(Self {
            signals: Signals::new(signals)?,
            waiting: false,
        })
    }

    pub fn wait_for_signal(&mut self) -> Result<i32> {
        #[cfg(any(target_os = "macos", target_os = "openbsd"))]
        {
            let mut sig = 0;
            unsafe {
                sigwait(&self.signals.mask(), &mut sig);
            }
            self.waiting = false;
            Ok(sig)
        }

        #[cfg(not(any(target_os = "macos", target_os = "openbsd")))]
        {
            let timeout = timespec {
                tv_sec: 1,
                tv_nsec: 0,
            };

            loop {
                let sig = if self.waiting {
                    unsafe {
                        sigtimedwait(&self.signals.mask(), std::ptr::null_mut(), &timeout)
                    }
                } else {
                    unsafe {
                        sigwaitinfo(&self.signals.mask(), std::ptr::null_mut())
                    }
                };

                if sig != -1 {
                    return Ok(sig);
                }

                let err = std::io::Error::last_os_error();
                match err.kind() {
                    std::io::ErrorKind::WouldBlock => {
                        self.waiting = false;
                        continue;
                    }
                    std::io::ErrorKind::Interrupted => continue,
                    _ => return Err(SquashError::Other(format!(
                        "sigtimedwait/sigwaitinfo failed: {}",
                        err
                    ))),
                }
            }
        }
    }

    pub fn set_waiting(&mut self, waiting: bool) {
        self.waiting = waiting;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_waiter_creation() {
        let waiter = SignalWaiter::new(&[libc::SIGINT, libc::SIGTERM]).unwrap();
        assert!(!waiter.waiting);
    }

    #[test]
    fn test_waiting_flag() {
        let mut waiter = SignalWaiter::new(&[libc::SIGINT, libc::SIGTERM]).unwrap();
        waiter.set_waiting(true);
        assert!(waiter.waiting);
        waiter.set_waiting(false);
        assert!(!waiter.waiting);
    }
} 