//! Announcing damage distributions before resolution (CR 601.2d).

use super::{CardLookup, Engine, ObjectId, Pending, PlanKind, PlayerId, SmallVec};

impl<L: CardLookup> Engine<L> {
    /// Asks how the triggered ability just put on the stack divides its
    /// damage, if it prints "damage divided as you choose"
    /// ([`baylee_cards_dsl::Effect::DealDamageDivided`]).
    ///
    /// CR 601.2d, which CR 603.3d applies to a triggered ability: the
    /// division is announced as it is put on the stack, after its targets,
    /// and each target gets at least 1. One target takes it all and is not
    /// asked; no target divides nothing. Asked only when the second instance
    /// of "target" did not ask first — no card prints both.
    ///
    /// Returns whether a question is now pending.
    pub(crate) fn ask_trigger_division(&mut self) -> bool {
        let Some(top) = self
            .state
            .zones
            .list(crate::zone::ZoneLocation::Stack)
            .last()
            .copied()
        else {
            return false;
        };
        let Some(total) = self.stack_divided_amount(top) else {
            return false;
        };
        let Some(obj) = self.state.object(top) else {
            return false;
        };
        let (controller, targets) = (obj.controller, obj.targets.clone());
        // Entries for abilities that have left the stack since are dead.
        let state = &mut self.state;
        let on_stack = state.zones.list(crate::zone::ZoneLocation::Stack);
        state.divided.retain(|(id, _)| on_stack.contains(id));
        if targets.is_empty() {
            return false;
        }
        self.ask_division(controller, top, targets, Vec::new(), total)
    }

    /// Asks the next target's share of a division, or, when only the last
    /// target is left, gives it the rest and writes the division down.
    /// Returns whether a question is now pending.
    pub(super) fn ask_division(
        &mut self,
        controller: PlayerId,
        on_stack: ObjectId,
        targets: SmallVec<[ObjectId; 2]>,
        mut shares: Vec<u32>,
        total: u32,
    ) -> bool {
        let given: u32 = shares.iter().sum();
        let left = total.saturating_sub(given);
        let asked = shares.len();
        if asked + 1 >= targets.len() {
            shares.push(left);
            self.state.capture_source_references();
            let division = shares
                .into_iter()
                .enumerate()
                .map(|(index, share)| {
                    (
                        self.state
                            .recorded_target_reference(
                                on_stack,
                                false,
                                u32::try_from(index).expect("target slot"),
                            )
                            .expect("announced target"),
                        share,
                    )
                })
                .collect();
            self.state.divided.push((on_stack, division));
            return false;
        }
        // Each target still to come needs at least 1 (CR 601.2d); the
        // `TargetReq` asks for no more targets than there is damage, so this
        // leaves at least 1 for this one.
        let after = u32::try_from(targets.len() - asked - 1).unwrap_or(u32::MAX);
        let max = left.saturating_sub(after).max(1);
        let reason = crate::choice::NumberPrompt::DivideDamage {
            target: targets[asked],
            index: u8::try_from(asked).unwrap_or(u8::MAX),
            of: u8::try_from(targets.len()).unwrap_or(u8::MAX),
            left,
        };
        self.pending_plan = Some(PlanKind::DivideDamage {
            on_stack,
            targets,
            shares,
            total,
        });
        self.pending = Pending::ChooseNumber {
            player: controller,
            min: 1,
            max,
            reason,
        };
        self.awaiting_answer = true;
        true
    }
}
