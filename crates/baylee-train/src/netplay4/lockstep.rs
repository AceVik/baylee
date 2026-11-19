//! A searched decision's rollouts stepped together (`eval4 --search-lockstep`):
//! each rollout runs on its own thread as before, but its net steps wait
//! until every rollout still playing waits on one, and then all of them go
//! to the net as one group ([`super::BatchedNet4::run_group`]). A group is
//! as large as the rollouts still playing, so an accelerator that pays only
//! for batches (a GPU, an iGPU, an NPU) gets calls of up to K × P rows
//! instead of whatever happened to queue up at the same moment.
//!
//! Which rows share a group is a function of the rollouts alone, never of
//! timing: a member's steps come in the order its rollout asks for them, and
//! a group is sent only once every live member has asked. A rollout that
//! ends leaves the group, and the others go on without it.
//!
//! The barrier ([`Lockstep`]) is generic so its logic is tested without a
//! model; [`LockstepSeat4`] is the one a rollout player steps through.

use std::collections::BTreeMap;
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Mutex, PoisonError};

use anyhow::anyhow;

use super::inference::StepRow;
use super::{BatchedNet4, StepOutputs};

/// Where a complete group goes: every waiting member's item, in member
/// order, each with the sender its answer goes back on.
type Sink<T, R> = Box<dyn Fn(Vec<(T, SyncSender<R>)>) + Send + Sync>;

/// The barrier of one lockstep group of members.
pub(super) struct Lockstep<T, R> {
    waiting: Mutex<Waiting<T, R>>,
    sink: Sink<T, R>,
}

/// The members still playing and the steps of those that wait.
struct Waiting<T, R> {
    live: usize,
    steps: BTreeMap<usize, (T, SyncSender<R>)>,
}

impl<T, R> Lockstep<T, R> {
    /// A group of `members`, each of which steps or leaves through a
    /// [`Member`] of its own.
    pub(super) fn new(
        members: usize,
        sink: impl Fn(Vec<(T, SyncSender<R>)>) + Send + Sync + 'static,
    ) -> Arc<Self> {
        Arc::new(Self {
            waiting: Mutex::new(Waiting {
                live: members,
                steps: BTreeMap::new(),
            }),
            sink: Box::new(sink),
        })
    }

    /// The group's members, numbered in the order the group sends them.
    pub(super) fn members(self: &Arc<Self>, count: usize) -> Vec<Member<T, R>> {
        (0..count)
            .map(|index| Member {
                lockstep: Arc::clone(self),
                index,
            })
            .collect()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Waiting<T, R>> {
        self.waiting.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Sends the group once every live member waits.
    fn send_if_complete(&self, waiting: &mut Waiting<T, R>) {
        if waiting.live > 0 && waiting.steps.len() == waiting.live {
            let group = std::mem::take(&mut waiting.steps).into_values().collect();
            (self.sink)(group);
        }
    }
}

/// One member of a [`Lockstep`] group: it steps through it until it is
/// dropped, which leaves the group.
pub(super) struct Member<T, R> {
    lockstep: Arc<Lockstep<T, R>>,
    index: usize,
}

impl<T, R> Member<T, R> {
    /// Waits with `item` until every live member waits, and returns what the
    /// sink answered for it; `None` when the sink dropped its sender.
    pub(super) fn step(&self, item: T) -> Option<R> {
        let (reply, answer) = mpsc::sync_channel(1);
        {
            let mut waiting = self.lockstep.lock();
            let earlier = waiting.steps.insert(self.index, (item, reply));
            debug_assert!(earlier.is_none(), "a member steps once at a time");
            self.lockstep.send_if_complete(&mut waiting);
        }
        answer.recv().ok()
    }
}

impl<T, R> Drop for Member<T, R> {
    /// Leaves the group: the others no longer wait for this member.
    fn drop(&mut self) {
        let mut waiting = self.lockstep.lock();
        waiting.live -= 1;
        self.lockstep.send_if_complete(&mut waiting);
    }
}

/// A rollout player's seat in a lockstep group over a batched net.
pub(super) struct LockstepSeat4 {
    member: Member<StepRow, anyhow::Result<StepOutputs>>,
    net: Arc<BatchedNet4>,
}

impl LockstepSeat4 {
    /// `count` seats of one group over `net`, in the order its steps go.
    pub(super) fn group(net: &Arc<BatchedNet4>, count: usize) -> Vec<Self> {
        let sending = Arc::clone(net);
        let lockstep = Lockstep::new(count, move |steps| sending.run_group(steps));
        lockstep
            .members(count)
            .into_iter()
            .map(|member| Self {
                member,
                net: Arc::clone(net),
            })
            .collect()
    }

    /// Runs `row` in the group's next call.
    pub(super) fn run(&self, row: StepRow) -> anyhow::Result<StepOutputs> {
        self.member
            .step(row)
            .unwrap_or_else(|| Err(anyhow!("the batched net stopped")))
    }

    /// The net the group runs on.
    pub(super) const fn net(&self) -> &Arc<BatchedNet4> {
        &self.net
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::Receiver;

    /// A group whose sink answers each item with itself plus one from a
    /// thread of its own, as a net's lane does, and notes every group's
    /// items.
    fn echo(members: usize) -> (Vec<Member<u64, u64>>, Receiver<Vec<u64>>) {
        let (noted, groups) = mpsc::channel();
        let noted = Mutex::new(noted);
        let lockstep = Lockstep::new(members, move |steps: Vec<(u64, SyncSender<u64>)>| {
            let items: Vec<u64> = steps.iter().map(|(item, _)| *item).collect();
            noted
                .lock()
                .unwrap()
                .send(items)
                .expect("the test reads the groups");
            std::thread::spawn(move || {
                for (item, reply) in steps {
                    reply.send(item + 1).unwrap();
                }
            });
        });
        (lockstep.members(members), groups)
    }

    #[test]
    fn every_live_member_steps_in_every_group_in_member_order() {
        // Member m steps m + 1 times; item = 100 × member + step.
        let (members, groups) = echo(4);
        std::thread::scope(|scope| {
            for (m, member) in members.into_iter().enumerate() {
                scope.spawn(move || {
                    for step in 0..=m as u64 {
                        let item = 100 * m as u64 + step;
                        assert_eq!(member.step(item), Some(item + 1));
                    }
                });
            }
        });
        let seen: Vec<Vec<u64>> = groups.try_iter().collect();
        assert_eq!(
            seen,
            [
                vec![0, 100, 200, 300],
                vec![101, 201, 301],
                vec![202, 302],
                vec![303]
            ],
            "a group holds one step of every member still playing, in member order"
        );
    }

    #[test]
    fn a_member_that_leaves_without_stepping_releases_the_others() {
        let (mut members, groups) = echo(3);
        let idle = members.pop().unwrap();
        std::thread::scope(|scope| {
            for (m, member) in members.into_iter().enumerate() {
                scope.spawn(move || assert_eq!(member.step(m as u64), Some(m as u64 + 1)));
            }
            drop(idle);
        });
        assert_eq!(groups.try_iter().collect::<Vec<_>>(), [vec![0, 1]]);
    }

    #[test]
    fn a_dropped_answer_is_none_not_a_hang() {
        let lockstep = Lockstep::new(1, |steps: Vec<(u64, SyncSender<u64>)>| drop(steps));
        let member = lockstep.members(1).pop().unwrap();
        assert_eq!(member.step(7), None);
    }
}
