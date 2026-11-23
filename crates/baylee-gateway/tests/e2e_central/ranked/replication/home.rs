//! The home lobby renews its own short-lived bearer while the duel runs elsewhere.
use super::{Player, Registered, common};
use std::{
    sync::mpsc::{self, RecvTimeoutError, Sender, SyncSender},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

pub(super) struct Session {
    queries: Option<Sender<SyncSender<String>>>,
    worker: Option<JoinHandle<()>>,
}

impl Session {
    pub(super) fn start(home: &Registered, player: &Player) -> Self {
        let port = home.gateway.port;
        let mut token = Zeroizing::new(player.home.token.clone());
        let first = player.home.renew_in.expect("initial home bearer renewal");
        let (queries, requests) = mpsc::channel::<SyncSender<String>>();
        let worker = thread::spawn(move || {
            let mut due = Instant::now() + Duration::from_secs(first);
            loop {
                match requests.recv_timeout(due.saturating_duration_since(Instant::now())) {
                    Ok(reply) => {
                        let (code, body) = common::http(port, "GET", "/me", Some(&token), "");
                        assert_eq!(code, 200, "home lobby session must survive remote play");
                        reply
                            .send(common::json_field(&body, "id").to_owned())
                            .unwrap();
                    }
                    Err(RecvTimeoutError::Timeout) => {
                        let (code, body) =
                            common::http(port, "POST", "/auth/session/renew", Some(&token), "");
                        assert_eq!(code, 200, "home gateway must renew the original session");
                        let mut renewed: baylee_account::Admission =
                            serde_json::from_str(&body).unwrap();
                        due = Instant::now() + Duration::from_secs(renewed.renew_in.unwrap());
                        token = Zeroizing::new(std::mem::take(&mut renewed.token));
                    }
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            }
        });
        Self {
            queries: Some(queries),
            worker: Some(worker),
        }
    }

    pub(super) fn identity(&self) -> String {
        let (reply, result) = mpsc::sync_channel(0);
        self.queries.as_ref().unwrap().send(reply).unwrap();
        result.recv().unwrap()
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.queries.take();
        if let Some(worker) = self.worker.take()
            && let Err(cause) = worker.join()
            && !thread::panicking()
        {
            std::panic::resume_unwind(cause);
        }
    }
}
