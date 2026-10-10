use super::*;
use std::sync::{Arc, Condvar};
use std::time::{Duration, Instant};

struct SlowHost {
    gate: Arc<(Mutex<bool>, Condvar)>,
    started: mpsc::Sender<()>,
    dropped: mpsc::Sender<()>,
    record: Vec<u8>,
    fresh: bool,
}

impl DuelHost for SlowHost {
    fn poll(&mut self) -> Vec<HostMessage> {
        if std::mem::take(&mut self.fresh) {
            vec![HostMessage::Spectators(self.record.len() as u32)]
        } else {
            vec![]
        }
    }
    fn submit(&mut self, _: PlayerAction) {
        self.started.send(()).unwrap();
        let (lock, condition) = &*self.gate;
        let _guard = condition
            .wait_while(lock.lock().unwrap(), |open| !*open)
            .unwrap();
        self.record.push(self.record.len() as u8);
        self.fresh = true;
    }
    fn ready(&mut self) {}
    fn seat(&self) -> PlayerId {
        PlayerId::new(0)
    }
    fn own_deck(&self) -> Vec<CardIndex> {
        vec![CardIndex::new(7)]
    }
    fn local_record(&self) -> Option<&[u8]> {
        Some(&self.record)
    }
    fn resume_point(&self) -> Option<ResumePoint> {
        Some(ResumePoint {
            record: "fixture".into(),
            seat: self.seat(),
            names: vec!["Player".into()],
            hash: self.record.len().to_string(),
        })
    }
}

impl Drop for SlowHost {
    fn drop(&mut self) {
        let _ = self.dropped.send(());
    }
}

fn next(host: &mut BackgroundHost) -> Vec<HostMessage> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let messages = host.poll();
        if !messages.is_empty() {
            return messages;
        }
        assert!(Instant::now() < deadline, "worker did not reply");
        std::thread::yield_now();
    }
}

#[test]
fn ui_keeps_polling_during_a_blocked_action_and_records_are_incremental() {
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    let (started, began) = mpsc::channel();
    let (dropped, ended) = mpsc::channel();
    let mut host = BackgroundHost::spawn(SlowHost {
        gate: gate.clone(),
        started,
        dropped,
        record: vec![0],
        fresh: true,
    });
    assert!(matches!(
        next(&mut host).as_slice(),
        [HostMessage::Spectators(1)]
    ));
    assert_eq!(host.own_deck(), [CardIndex::new(7)]);
    assert_eq!(host.local_record(), Some([0].as_slice()));
    host.submit(PlayerAction::PassPriority);
    began.recv_timeout(Duration::from_secs(10)).unwrap();
    // The worker cannot finish until this test releases it. Neither submitting
    // more ordered actions nor polling may wait for it.
    for _ in 0..3 {
        host.submit(PlayerAction::PassPriority);
    }
    for _ in 0..100 {
        assert!(host.poll().is_empty());
    }
    assert!(host.resume_point().is_none());
    assert!(host.busy());
    assert_eq!(host.local_record(), Some([0].as_slice()));
    *gate.0.lock().unwrap() = true;
    gate.1.notify_one();
    for count in 2..=5 {
        assert!(matches!(next(&mut host).as_slice(), [HostMessage::Spectators(n)] if *n == count));
    }
    assert_eq!(host.local_record(), Some([0, 1, 2, 3, 4].as_slice()));
    assert_eq!(host.resume_point().unwrap().hash, "5");
    assert!(!host.busy());
    drop(host);
    ended.recv_timeout(Duration::from_secs(10)).unwrap();
}

#[test]
fn leaving_table_does_not_join_a_busy_worker() {
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    let (started, began) = mpsc::channel();
    let (dropped, ended) = mpsc::channel();
    let mut host = BackgroundHost::spawn(SlowHost {
        gate: gate.clone(),
        started,
        dropped,
        record: vec![0],
        fresh: true,
    });
    next(&mut host);
    host.submit(PlayerAction::PassPriority);
    began.recv_timeout(Duration::from_secs(10)).unwrap();
    drop(host);
    *gate.0.lock().unwrap() = true;
    gate.1.notify_one();
    ended.recv_timeout(Duration::from_secs(10)).unwrap();
}

#[test]
fn real_worker_matches_synchronous_play_and_its_record_resumes() {
    let preset = crate::host::tests::duel_preset();
    let seat = PlayerId::new(0);
    let names = ["You", "House"];
    let mut reference = LocalHost::new(&preset, seat, &names).unwrap();
    let mut worker = BackgroundHost::new(LocalHost::new(&preset, seat, &names).unwrap());
    reference.poll();
    assert!(
        next(&mut worker)
            .iter()
            .any(|m| matches!(m, HostMessage::Static(_)))
    );
    for _ in 0..16 {
        assert_eq!(worker.resume_point().unwrap().hash, reference.hash());
        let Some(action) = reference.session.house_action(seat) else {
            break;
        };
        worker.submit(action.clone());
        reference.submit(action);
        reference.poll();
        let messages = next(&mut worker);
        assert!(!messages.iter().any(|m| matches!(m, HostMessage::Failed(_))));
        assert!(messages.iter().any(|m| matches!(m, HostMessage::View(..))));
    }
    let point = worker.resume_point().unwrap();
    assert_eq!(point.hash, reference.hash());
    let resumed = LocalHost::resume(
        worker.local_record().unwrap().to_vec(),
        &point.record,
        seat,
        &point.names,
    )
    .unwrap();
    assert_eq!(resumed.hash(), point.hash);
    assert_eq!(
        format!("{:?}", resumed.session.pending()),
        format!("{:?}", reference.session.pending())
    );
}

#[test]
fn polling_continues_a_budget_exhausted_house_turn() {
    let preset = crate::host::tests::duel_preset();
    let seat = PlayerId::new(0);
    let mut host = LocalHost::new(&preset, seat, &["You", "House"]).unwrap();
    // Simulate a prior pump returning at its budget before the AI's mulligan.
    host.statics = None;
    assert!(host.session.pump_at_most(0).is_empty());
    assert!(host.busy());
    let messages = host.poll();
    assert!(!host.busy());
    assert!(messages.iter().any(|m| matches!(m, HostMessage::Choice(_))));
    assert!(messages.iter().any(|m| matches!(m, HostMessage::View(..))));
}
