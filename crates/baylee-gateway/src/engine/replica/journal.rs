//! Apply one authenticated intent, wait for independent durable certification,
//! then release its seat views. Rehydration compares every historical witness.
use super::{Head, Msg, Role, Run, Shared, VecDeque, mpsc, replicate, v1};
use crate::central::Bridge;
use central_api::ranked::{
    Outcome,
    replica::{Entry, Evidence, Fence, HostRequest as Request, HostResponse as Response, Source},
};
use std::time::Duration;
use tokio::sync::watch;

struct Page {
    initial: Option<Evidence>,
    entries: VecDeque<Entry>,
    certified: Head,
}

async fn page(bridge: &Bridge, run: &Run, after: u64) -> Option<Page> {
    let response = replicate::rpc(
        bridge,
        Request::Tail {
            fence: run.work.fence,
            after,
        },
    )
    .await
    .ok()?;
    let Response::Page {
        opening,
        initial,
        entries,
        certified,
        ..
    } = response
    else {
        return None;
    };
    if opening != run.opening || certified.position < after {
        return None;
    }
    Some(Page {
        initial,
        entries: entries.into(),
        certified,
    })
}

async fn attest(bridge: &Bridge, fence: Fence, head: Head, proof: Evidence) -> bool {
    let req = Request::Attest {
        fence,
        head,
        state: proof.state,
        accepted: proof.accepted,
        awaited: proof.awaited,
        decisions: proof.decisions,
        terminal: proof.terminal,
    };
    matches!(
        replicate::rpc(bridge, req).await,
        Ok(Response::Done | Response::Assignment { .. })
    )
}

fn evidence(vote: &v1::RankedEngineVote) -> Option<Evidence> {
    if vote.awaiting > 3 {
        return None;
    }
    let terminal = match vote.outcome {
        0 => None,
        1 => Some(Outcome::FirstWins),
        2 => Some(Outcome::SecondWins),
        3 => Some(Outcome::Draw),
        _ => return None,
    };
    Some(Evidence {
        state: vote.evidence.as_slice().try_into().ok()?,
        accepted: vote.accepted,
        awaited: u8::try_from(vote.awaiting).ok()?,
        decisions: [vote.asked_zero, vote.asked_one],
        terminal,
    })
}

fn step(fence: Fence, parent: Head, entry: &Entry) -> Option<Msg> {
    let next = parent
        .next(fence.match_id, entry.command_id, entry.source, &entry.body)
        .ok()?;
    if entry.head != next {
        return None;
    }
    let (seat, authority) = match entry.source {
        Source::Seat0 => (0, false),
        Source::Seat1 => (1, false),
        Source::Authority => (
            u32::from(central_api::ranked::replica::clock::ClockEvent::parse(&entry.body)?.seat()),
            true,
        ),
    };
    Some(Msg::RankedEngineStep(v1::RankedEngineStep {
        generation: fence.generation,
        position: next.position,
        command_id: entry.command_id.to_vec(),
        operation: entry.body.as_slice().to_vec(),
        seat,
        authority,
    }))
}

struct Execution<'a> {
    state: &'a Shared,
    run: &'a Run,
    bridge: &'a Bridge,
    orders: mpsc::Sender<Msg>,
    votes: &'a mut mpsc::Receiver<v1::RankedEngineVote>,
    snapshots: &'a mut watch::Receiver<[u64; 2]>,
    served: [u64; 2],
    last_snapshot: [Option<std::time::Instant>; 2],
}

pub(super) async fn drive(
    state: &Shared,
    bridge: &Bridge,
    run: &Run,
    orders: mpsc::Sender<Msg>,
    votes: &mut mpsc::Receiver<v1::RankedEngineVote>,
    snapshots: &mut watch::Receiver<[u64; 2]>,
) {
    let mut job = Execution {
        state,
        run,
        bridge,
        orders,
        votes,
        snapshots,
        served: [0; 2],
        last_snapshot: [None; 2],
    };
    let _ = job.apply().await;
}

impl Execution<'_> {
    async fn fetch(&self, after: u64) -> Option<Page> {
        while self.run.is_current(self.state) {
            if let Some(page) = page(self.bridge, self.run, after).await {
                return Some(page);
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        None
    }

    async fn apply(&mut self) -> Option<()> {
        let run = self.run;
        let fence = run.work.fence;
        let setup = Msg::RankedEngineSetup(v1::RankedEngineSetup {
            match_id: fence.match_id.to_vec(),
            generation: fence.generation,
            primary: run.work.role == Role::Primary,
            setup: Some(run.setup.clone()),
        });
        self.orders.send(setup).await.ok()?;
        let mut head = run.opening;
        let mut page = self.fetch(0).await?;
        let mut known = page.initial;
        let mut parent = head;
        let mut id = None;
        loop {
            let vote = tokio::time::timeout(Duration::from_secs(30), self.votes.recv())
                .await
                .ok()??;
            if !run.is_current(self.state)
                || vote.generation != fence.generation
                || vote.position != head.position
                || vote.command_id.as_slice()
                    != id.as_ref().map_or(&[][..], |id: &[u8; 16]| id.as_slice())
            {
                return None;
            }
            let proof = evidence(&vote)?;
            if known.is_some_and(|old| old != proof) {
                attest(self.bridge, fence, head, proof).await;
                return None;
            }
            let replay = known.is_some();
            if !replay || page.certified == head {
                // On a new term both engines also prove they recovered the
                // entire already-certified prefix before the first live step.
                let mut sent = false;
                while !sent && run.is_current(self.state) {
                    sent = attest(self.bridge, fence, head, proof).await;
                    if !sent {
                        tokio::time::sleep(Duration::from_millis(350)).await;
                    }
                }
                if !sent {
                    return None;
                }
            }
            if !replay {
                loop {
                    page = self.fetch(parent.position).await?;
                    let certified = if id.is_none() {
                        page.initial
                    } else {
                        page.entries
                            .front()
                            .filter(|entry| entry.head == head)
                            .and_then(|entry| entry.evidence)
                    };
                    if let Some(certified) = certified {
                        if certified != proof || page.certified.position < head.position {
                            return None;
                        }
                        if id.is_some() {
                            page.entries.pop_front();
                        }
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(150)).await;
                }
            }
            let publish = !replay && run.work.role == Role::Primary && run.deadline().is_some();
            self.orders
                .send(Msg::RankedEngineCommit(v1::RankedEngineCommit {
                    generation: fence.generation,
                    position: head.position,
                    evidence: proof.state.to_vec(),
                    chain: head.chain.to_vec(),
                    publish,
                }))
                .await
                .ok()?;
            if page.certified == head {
                *run.engine_head.lock() = Some(head);
                run.activate(self.state, head);
            }
            if proof.terminal.is_some() {
                run.terminal
                    .store(true, std::sync::atomic::Ordering::Release);
                // The engine's final per-seat frames follow its commit. Leave
                // the router attached until the privileged worker retires it.
                while run.is_current(self.state) {
                    tokio::time::sleep(Duration::from_millis(250)).await;
                }
                return Some(());
            }
            let entry = self.next_entry(head, &mut page).await?;
            let command = step(fence, head, &entry)?;
            parent = head;
            head = entry.head;
            known = entry.evidence;
            id = Some(entry.command_id);
            self.orders.send(command).await.ok()?;
        }
    }

    async fn next_entry(&mut self, head: Head, page: &mut Page) -> Option<Entry> {
        let mut timer = tokio::time::interval(Duration::from_millis(250));
        timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            if let Some(entry) = page.entries.pop_front() {
                return Some(entry);
            }
            self.serve_snapshots(head).await?;
            tokio::select! {
                _ = timer.tick() => { *page = self.fetch(head.position).await?; }
                changed = self.snapshots.changed() => { changed.ok()?; }
            }
        }
    }

    async fn serve_snapshots(&mut self, head: Head) -> Option<()> {
        if self.run.work.role != Role::Primary || self.run.deadline().is_none() {
            return Some(());
        }
        let new = *self.snapshots.borrow_and_update();
        for (seat, count) in new.into_iter().enumerate() {
            if count != self.served[seat]
                && self.last_snapshot[seat]
                    .is_none_or(|at| at.elapsed() >= Duration::from_millis(500))
            {
                self.orders
                    .send(Msg::RankedEngineSnapshot(v1::RankedEngineSnapshot {
                        generation: self.run.work.fence.generation,
                        position: head.position,
                        seat: u32::try_from(seat).ok()?,
                    }))
                    .await
                    .ok()?;
                self.last_snapshot[seat] = Some(std::time::Instant::now());
                self.served[seat] = count;
            }
        }
        Some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use central_api::ranked::replica::clock::{ClockEvent, Reason};

    #[test]
    fn stepping_checks_the_parent_and_uses_only_the_authenticated_source() {
        let fence = Fence {
            match_id: [17; 16],
            generation: 4,
        };
        let parent = Head {
            position: 3,
            chain: [22; 32],
        };
        let id = [91; 16];
        let body = ClockEvent::new(0, 7, Reason::Disconnected)
            .unwrap()
            .encode()
            .unwrap();
        let entry = |source| Entry {
            command_id: id,
            head: parent.next(fence.match_id, id, source, &body).unwrap(),
            source,
            body: body.clone(),
            evidence: None,
        };
        let mut seat_command = entry(Source::Seat1);
        let Some(Msg::RankedEngineStep(user)) = step(fence, parent, &seat_command) else {
            panic!("seat step missing");
        };
        assert_eq!(
            (user.generation, user.position, user.seat, user.authority),
            (4, 4, 1, false)
        );
        assert_eq!(user.operation, body.as_slice());
        let Some(Msg::RankedEngineStep(clock)) = step(fence, parent, &entry(Source::Authority))
        else {
            panic!("clock step missing");
        };
        assert_eq!((clock.seat, clock.authority), (0, true));
        seat_command.source = Source::Authority;
        assert!(step(fence, parent, &seat_command).is_none());
        assert!(
            step(
                fence,
                Head {
                    position: 4,
                    ..parent
                },
                &entry(Source::Seat1)
            )
            .is_none()
        );
    }
}
