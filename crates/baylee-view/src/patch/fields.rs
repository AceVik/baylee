//! The fields of a [`PlayerView`] that are neither a zone nor a seat line, each
//! written down once: the macro below makes from this one list the variant that
//! carries a field, the comparison that finds it changed and the assignment
//! that sets it. Its last line refuses to compile while a field of the view is
//! in no list, so a field added to the view cannot silently stay out of a patch.

use crate::combat::CombatView;
use crate::log::PolicyAct;
use crate::objects::{DamageSourceView, HandObject, OrderedTrigger, PublicObject};
use crate::patch::{ListEdit, PatchError, Zone};
use crate::player_view::{
    Ballot, Division, LostSeat, PlayerControl, PlayerView, SeatClock, TargetingContext,
};
use crate::seats::SeatView;
use crate::shown_hands::SharedHand;
use crate::turn::{DayNight, Phase, Step};
use baylee_core::ids::{ObjectId, PlayerId, SeatSet};
use baylee_core::mana::ManaPayment;
use baylee_core::types::TypeSet;
use serde::{Deserialize, Serialize};

macro_rules! view_fields {
    ($($variant:ident $field:ident: $ty:ty,)*) => {
        /// A field of the view with the value it has now. A patch holds few of
        /// these, so one the size of an object costs nothing to speak of.
        #[allow(clippy::large_enum_variant)]
        #[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
        pub enum ViewField {
            $(
                #[doc = concat!("[`PlayerView::", stringify!($field), "`].")]
                $variant($ty),
            )*
            /// [`PlayerView::target_objects`], against the last one: a long
            /// stack aims at hundreds, and one more should not cost them all.
            TargetObjects(ListEdit<DamageSourceView>),
            /// [`PlayerView::policy_acts`], against the last one: a window that
            /// slides by one, sixteen entries wide.
            PolicyActs(ListEdit<PolicyAct>),
            /// A zone whole, for the view whose ids do not name its objects
            /// because one comes up twice.
            Zone(Zone, Vec<PublicObject>),
            /// The hand whole, for the same reason.
            Hand(Vec<HandObject>),
            /// Every seat line, when the seats themselves are not the last
            /// view's.
            Seats(Vec<SeatView>),
            /// Every graveyard, when the seats are not the last view's.
            Graveyards(Vec<Vec<PublicObject>>),
            /// Every exile, when the seats are not the last view's.
            Exile(Vec<Vec<PublicObject>>),
            /// Every command zone, when the seats are not the last view's.
            Command(Vec<Vec<PublicObject>>),
        }

        /// The fields of this list and the two edited lists that differ, with
        /// what `new` has.
        pub(super) fn changed(old: &PlayerView, new: &PlayerView, out: &mut Vec<ViewField>) {
            $(
                if old.$field != new.$field {
                    out.push(ViewField::$variant(new.$field.clone()));
                }
            )*
            if old.target_objects != new.target_objects {
                let edit = ListEdit::of(&old.target_objects, &new.target_objects);
                out.push(ViewField::TargetObjects(edit));
            }
            if old.policy_acts != new.policy_acts {
                let edit = ListEdit::of(&old.policy_acts, &new.policy_acts);
                out.push(ViewField::PolicyActs(edit));
            }
        }

        impl ViewField {
            /// Sets the field on `view`.
            pub(super) fn set(&self, view: &mut PlayerView) -> Result<(), PatchError> {
                match self {
                    $(Self::$variant(value) => view.$field.clone_from(value),)*
                    Self::TargetObjects(edit) => {
                        view.target_objects = edit.applied(&view.target_objects)?;
                    }
                    Self::PolicyActs(edit) => view.policy_acts = edit.applied(&view.policy_acts)?,
                    Self::Zone(zone, objects) => view
                        .zone_mut(*zone)
                        .ok_or(PatchError::NoZone(*zone))?
                        .clone_from(objects),
                    Self::Hand(cards) => view.hand.clone_from(cards),
                    Self::Seats(seats) => view.seats.clone_from(seats),
                    Self::Graveyards(zones) => view.graveyards.clone_from(zones),
                    Self::Exile(zones) => view.exile.clone_from(zones),
                    Self::Command(zones) => view.command.clone_from(zones),
                }
                Ok(())
            }
        }

        /// Every field of the view is in the list above or is one of the
        /// others a patch carries: a new one fails to compile here.
        const _: fn(&PlayerView) = |view| {
            let PlayerView {
                $($field: _,)*
                seq: _,
                seat: _,
                target_objects: _,
                policy_acts: _,
                seats: _,
                hand: _,
                battlefield: _,
                stack: _,
                graveyards: _,
                exile: _,
                command: _,
                looking_at: _,
                library_tops: _,
            } = view;
        };
    };
}

view_fields! {
    DecisionPlayer decision_player: Option<PlayerId>,
    ControlledHands controlled_hands: Vec<SharedHand>,
    DamageSources damage_sources: Vec<DamageSourceView>,
    Targeting targeting: Option<TargetingContext>,
    Casting casting: Option<ObjectId>,
    AnnouncedCast announced_cast: Option<ObjectId>,
    AnnouncedX announced_x: Option<u32>,
    Turn turn: u32,
    Phase phase: Phase,
    Step step: Step,
    Active active: PlayerId,
    Awaiting awaiting: Option<PlayerId>,
    Deciding deciding: SeatSet,
    DecisionRemainingMs decision_remaining_ms: Option<u32>,
    Clocks clocks: Vec<SeatClock>,
    Lost lost: Vec<LostSeat>,
    PriorityHeld priority_held: bool,
    Owed owed: Option<ManaPayment>,
    Monarch monarch: Option<PlayerId>,
    DayNight day_night: Option<DayNight>,
    SharedHands shared_hands: Vec<SharedHand>,
    HandSharedWith hand_shared_with: SeatSet,
    HandRequests hand_requests: SeatSet,
    HandRequested hand_requested: SeatSet,
    Combat combat: CombatView,
    Ballots ballots: Vec<Ballot>,
    TriggerOrder trigger_order: Vec<OrderedTrigger>,
    SorceryLock sorcery_lock: Option<ObjectId>,
    SorceriesHaveFlash sorceries_have_flash: bool,
    FlashTypes flash_types: TypeSet,
    Controls controls: Vec<PlayerControl>,
    ForeignTargets foreign_targets: Vec<ObjectId>,
    Divided divided: Vec<Division>,
}
