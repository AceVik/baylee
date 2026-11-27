//! Every signal a process can be asked to stop by ([`Stoppers`]), shared by
//! `baylee-seat` and every other program that plays a seat (the ranked
//! runner's `ranked::cli`, the trained net's `seat4`).

/// Every signal a process can be asked to stop by, registered with the
/// operating system the moment this is built (`arm`), well before anything
/// reserves a game's budget: ctrl-c; on unix also SIGTERM (`kill`,
/// `docker stop`, a service manager); on Windows also ctrl-break, its
/// console closing, and the user logging off or the machine shutting down,
/// where the system waits a few seconds for the process to finish. One
/// that cannot be registered (rare: an operating-system limit) is simply
/// never answered, as before. SIGHUP keeps its own meaning (a bridge under
/// `nohup` plays on), and SIGKILL can never be answered: a bridge stopped
/// by either counts its reservation in full.
#[cfg_attr(windows, allow(clippy::struct_field_names))] // Windows names them all `Ctrl…`
pub struct Stoppers {
    #[cfg(unix)]
    interrupt: Option<tokio::signal::unix::Signal>,
    #[cfg(unix)]
    terminate: Option<tokio::signal::unix::Signal>,
    #[cfg(windows)]
    ctrl_c: Option<tokio::signal::windows::CtrlC>,
    #[cfg(windows)]
    ctrl_break: Option<tokio::signal::windows::CtrlBreak>,
    #[cfg(windows)]
    ctrl_close: Option<tokio::signal::windows::CtrlClose>,
    #[cfg(windows)]
    ctrl_logoff: Option<tokio::signal::windows::CtrlLogoff>,
    #[cfg(windows)]
    ctrl_shutdown: Option<tokio::signal::windows::CtrlShutdown>,
}

impl Stoppers {
    /// Registers every listener now, synchronously (each constructor here
    /// installs the operating system's own hook before returning); `None`
    /// only where that failed.
    #[must_use]
    pub fn arm() -> Self {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{SignalKind, signal};
            Self {
                interrupt: signal(SignalKind::interrupt()).ok(),
                terminate: signal(SignalKind::terminate()).ok(),
            }
        }
        #[cfg(windows)]
        {
            use tokio::signal::windows::{
                ctrl_break, ctrl_c, ctrl_close, ctrl_logoff, ctrl_shutdown,
            };
            Self {
                ctrl_c: ctrl_c().ok(),
                ctrl_break: ctrl_break().ok(),
                ctrl_close: ctrl_close().ok(),
                ctrl_logoff: ctrl_logoff().ok(),
                ctrl_shutdown: ctrl_shutdown().ok(),
            }
        }
        #[cfg(not(any(unix, windows)))]
        Self {}
    }

    /// The first of these already-armed listeners to fire; forever on one
    /// that was never armed.
    pub async fn stopped(&mut self) -> &'static str {
        #[cfg(unix)]
        {
            /// Waits on an armed listener, or forever if it was never
            /// armed (`arm` failed to register it).
            async fn on(
                listener: &mut Option<tokio::signal::unix::Signal>,
                name: &'static str,
            ) -> &'static str {
                match listener {
                    Some(listener) => {
                        listener.recv().await;
                        name
                    }
                    None => std::future::pending().await,
                }
            }
            let Self {
                interrupt,
                terminate,
            } = self;
            tokio::select! {
                by = on(interrupt, "ctrl-c") => by,
                by = on(terminate, "SIGTERM") => by,
            }
        }
        #[cfg(windows)]
        {
            let Self {
                ctrl_c,
                ctrl_break,
                ctrl_close,
                ctrl_logoff,
                ctrl_shutdown,
            } = self;
            macro_rules! on {
                ($listener:expr, $name:expr) => {
                    async {
                        match $listener {
                            Some(listener) => {
                                listener.recv().await;
                                $name
                            }
                            None => std::future::pending().await,
                        }
                    }
                };
            }
            tokio::select! {
                by = on!(ctrl_c, "ctrl-c") => by,
                by = on!(ctrl_break, "ctrl-break") => by,
                by = on!(ctrl_close, "the console closing") => by,
                by = on!(ctrl_logoff, "the user logging off") => by,
                by = on!(ctrl_shutdown, "the machine shutting down") => by,
            }
        }
        #[cfg(not(any(unix, windows)))]
        {
            match tokio::signal::ctrl_c().await {
                Ok(_) => "ctrl-c",
                Err(_) => std::future::pending().await,
            }
        }
    }
}
