//! The states that lie on a card while they last (`cardmat::veil`): how a card's track moves between them, what the look and the material carry of it, and what the table asks of the question and the status to name them (`placement::veil_of`).

use crate::cardmat::{self, CardLook, Veil, VeilTrack, veil};
use crate::{Duel, rebuild_board};
use baylee_client_core::board::CardGroup;
use baylee_client_core::images::{ArtSize, FinishTreatment, ImageKey};
use baylee_client_core::test_support::{ViewBuilder, token};
use baylee_core::ids::{ObjectId, PlayerId, PrintRef};
use baylee_engine::choice::{Pending, TargetPrompt};
use baylee_view::ObjectStatus;
use bevy::prelude::Color;

/// Two floats equal to within what a division by a fade time can lose.
fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
}

// ------------------------------------------------------------ the track

/// A card that is already where it is wanted stays exactly as it was: the
/// start time of a fade in flight is not renewed by asking again.
#[test]
fn a_track_toward_the_states_it_is_going_to_is_unchanged() {
    let dimming = VeilTrack::new(0).toward(veil::DIM, 1.0, true);
    assert_eq!(dimming.toward(veil::DIM, 1.1, true), dimming);
    assert_eq!(VeilTrack::new(0).toward(0, 4.0, true), VeilTrack::new(0));
}

/// A card first drawn in a state is already in it: nothing on screen changed.
#[test]
fn a_new_track_is_settled() {
    let track = VeilTrack::new(veil::PHASED);
    assert_eq!(track.to, veil::PHASED);
    assert_eq!(track.from, veil::PHASED);
    assert_eq!(track.rate.to_bits(), 0);
    assert_eq!(track.veil(100.0), Veil::settled(veil::PHASED));
}

/// Dimming comes on over `DIM_FADE`, from the states the card was in.
#[test]
fn dimming_comes_on_over_the_short_fade() {
    let track = VeilTrack::new(0).toward(veil::DIM, 2.0, true);
    assert_eq!(track.to, veil::DIM);
    assert_eq!(track.from, 0);
    assert!(close(track.at, 2.0));
    assert!(close(track.rate, 1.0 / veil::DIM_FADE));
}

/// Dimming goes at once: the board is the player's again the moment the
/// question is answered, so the track comes back settled with no fade.
#[test]
fn dimming_leaves_at_once() {
    let dimmed = VeilTrack::new(0).toward(veil::DIM, 2.0, true);
    let answered = dimmed.toward(0, 2.05, true);
    assert_eq!(answered, VeilTrack::new(0));
    assert_eq!(answered.veil(2.06), Veil::settled(0));
}

/// Dimming that leaves while another state stays changes nothing else: no
/// fade is started for the state that did not move.
#[test]
fn dimming_leaving_a_card_that_stays_phased_out_starts_no_fade() {
    let both = VeilTrack::new(veil::DIM | veil::PHASED);
    assert_eq!(
        both.toward(veil::PHASED, 3.0, true),
        VeilTrack::new(veil::PHASED)
    );
}

/// A dimmed card that phases out fades in the new state from undimmed: the
/// dimming it is leaving is not faded out beside it.
#[test]
fn a_dimmed_card_phasing_out_fades_from_undimmed() {
    let track = VeilTrack::new(veil::DIM).toward(veil::PHASED, 5.0, true);
    assert_eq!(track.to, veil::PHASED);
    assert_eq!(track.from, 0);
    assert!(close(track.rate, 1.0 / veil::SLOW_FADE));
}

/// Phasing and sleep, coming or going, take the slow fade.
#[test]
fn phasing_and_sleep_take_the_slow_fade() {
    for (from, to) in [
        (0, veil::PHASED),
        (veil::PHASED, 0),
        (0, veil::ASLEEP),
        (veil::ASLEEP, 0),
    ] {
        let track = VeilTrack::new(from).toward(to, 1.0, true);
        assert!(close(track.rate, 1.0 / veil::SLOW_FADE), "{from} -> {to}");
        assert_eq!(track.from, from);
        assert_eq!(track.to, to);
    }
}

/// Only dimming coming on takes the short fade, though the card sleeps.
#[test]
fn dimming_a_sleeping_card_takes_the_short_fade() {
    let track = VeilTrack::new(veil::ASLEEP).toward(veil::ASLEEP | veil::DIM, 1.0, true);
    assert!(close(track.rate, 1.0 / veil::DIM_FADE));
    assert_eq!(track.from, veil::ASLEEP);
}

/// A sleeping card whose dimming lifts (the question was answered) stays
/// asleep and starts no fade: the part that did not move is not re-faded.
#[test]
fn a_sleeping_card_losing_its_dimming_starts_no_fade() {
    let both = VeilTrack::new(veil::ASLEEP | veil::DIM);
    assert_eq!(
        both.toward(veil::ASLEEP, 3.0, true),
        VeilTrack::new(veil::ASLEEP)
    );
}

/// A sleeping, dimmed card that wakes fades only the sleep out over the slow
/// fade; the dimming it leaves goes at once and is not faded beside it.
#[test]
fn waking_a_dimmed_sleeping_card_fades_only_the_sleep() {
    let track = VeilTrack::new(veil::ASLEEP | veil::DIM).toward(0, 1.0, true);
    assert_eq!(track.from, veil::ASLEEP);
    assert_eq!(track.to, 0);
    assert!(close(track.rate, 1.0 / veil::SLOW_FADE));
}

/// With motion off every change is made at once.
#[test]
fn without_motion_every_change_is_settled() {
    for to in [
        veil::DIM,
        veil::PHASED,
        veil::ASLEEP,
        veil::PHASED | veil::ASLEEP,
    ] {
        let track = VeilTrack::new(0).toward(to, 1.0, false);
        assert_eq!(track.from, track.to);
        assert_eq!(track.to, to);
        assert_eq!(track.rate.to_bits(), 0);
    }
}

/// A fade is transient while it lasts and settled once it has run: the card
/// then shares its look with every other card in the same states.
#[test]
fn a_fade_is_transient_until_it_has_run() {
    let track = VeilTrack::new(0).toward(veil::PHASED, 10.0, true);
    let mid = track.veil(10.0 + veil::SLOW_FADE * 0.5);
    assert_eq!(mid.to, veil::PHASED);
    assert_eq!(mid.from, 0);
    assert_ne!(mid.at, 0, "set out at a time other than zero");
    assert_ne!(mid.rate, 0);
    assert!(close(f32::from_bits(mid.at), 10.0));
    assert!(close(f32::from_bits(mid.rate), 1.0 / veil::SLOW_FADE));
    assert_ne!(mid, Veil::settled(veil::PHASED));

    let after = track.veil(10.0 + veil::SLOW_FADE + 0.01);
    assert_eq!(after, Veil::settled(veil::PHASED));
}

// ---------------------------------------------------- the look and the material

/// The shader's word names the states it is going to in the low byte and
/// the ones it comes from in the next.
#[test]
fn the_veil_code_packs_to_and_from() {
    let v = Veil {
        to: veil::PHASED | veil::DIM,
        from: veil::ASLEEP,
        at: 0,
        rate: 0,
    };
    assert_eq!(v.code(), 3 | (4 << 8));
    assert_eq!(Veil::settled(veil::ASLEEP).code(), 4 | (4 << 8));
    assert_eq!(Veil::NONE.code(), 0);
}

/// A material carries its look's veil: the code, and when and how fast.
#[test]
fn the_material_copies_the_looks_veil() {
    let track = VeilTrack::new(0).toward(veil::PHASED, 7.0, true);
    let look = CardLook::flat(Color::WHITE, FinishTreatment::Plain).with_veil(track.veil(7.1));
    let made = cardmat::material(look, None, Color::WHITE, 0.0);
    assert_eq!(made.params.veil, look.veil.code());
    assert_eq!(made.params.veil_at.to_bits(), look.veil.at);
    assert_eq!(made.params.veil_rate.to_bits(), look.veil.rate);
    assert!(close(made.params.veil_rate, 1.0 / veil::SLOW_FADE));
}

/// A card in no state draws as it did before there were any: every
/// constructor gives `Veil::NONE`, and its material carries zero.
#[test]
fn a_card_in_no_state_has_no_veil_and_a_plain_material() {
    let key = ImageKey::new(PrintRef::new(0), 0, ArtSize::Small);
    let looks = [
        CardLook::art(key, FinishTreatment::Plain),
        CardLook::flat(Color::WHITE, FinishTreatment::Plain),
        CardLook::back(FinishTreatment::Plain),
    ];
    for look in looks {
        assert_eq!(look.veil, Veil::NONE);
        let made = cardmat::material(look, None, Color::WHITE, 0.0);
        assert_eq!(made.params.veil, 0);
        assert_eq!(made.params.veil_at.to_bits(), 0);
        assert_eq!(made.params.veil_rate.to_bits(), 0);
    }
}

// ------------------------------------------------------------- veil_of

/// Three differently named permanents (so none piles with another) on this
/// seat's side, the second phased out, with the board built.
fn table() -> Duel {
    let mut phased = token(2, 0, "Phased", 1, 1);
    phased.status = ObjectStatus::PHASED_OUT;
    let view = ViewBuilder::new(2)
        .with_battlefield(
            0,
            [
                token(1, 0, "Alpha", 1, 1),
                phased,
                token(3, 0, "Gamma", 1, 1),
            ],
        )
        .build();
    let mut duel = Duel::default();
    duel.receive_view(view);
    rebuild_board(&mut duel);
    duel
}

fn id(slot: u32) -> ObjectId {
    ObjectId::new(slot, 0)
}

/// The group the board draws `object` in.
fn group_of(duel: &Duel, object: ObjectId) -> CardGroup {
    duel.board
        .as_ref()
        .expect("the board")
        .pods
        .iter()
        .flat_map(|pod| pod.lanes.iter())
        .flat_map(|lane| lane.groups.iter())
        .find(|g| g.members.contains(&object))
        .cloned()
        .expect("the object is drawn")
}

/// A question over `options`, asked of this seat or of another.
fn ask(duel: &mut Duel, player: u8, options: Vec<ObjectId>) {
    duel.receive_choice(Pending::ChooseTargets {
        player: PlayerId::new(player),
        options,
        player_options: Vec::new(),
        min: 1,
        max: 1,
        reason: TargetPrompt::Targets,
    });
}

/// With no question open nothing is dimmed, and a card phased out is named
/// phased.
#[test]
fn with_no_question_open_only_a_phased_out_card_has_a_veil() {
    let duel = table();
    assert!(duel.interaction.is_none());
    assert_eq!(super::veil_of(&duel, &group_of(&duel, id(1))), 0);
    assert_eq!(super::veil_of(&duel, &group_of(&duel, id(2))), veil::PHASED);
}

/// A question that offers some other permanent dims the ones it leaves out.
#[test]
fn a_question_dims_the_permanents_it_leaves_out() {
    let mut duel = table();
    ask(&mut duel, 0, vec![id(3)]);
    assert_eq!(super::veil_of(&duel, &group_of(&duel, id(1))), veil::DIM);
    assert_eq!(super::veil_of(&duel, &group_of(&duel, id(3))), 0, "offered");
}

/// A card phased out is phased even when the question leaves it out: it is
/// no choice and already says so, so dimming is not laid over it.
#[test]
fn a_phased_out_card_the_question_leaves_out_is_phased_not_dimmed() {
    let mut duel = table();
    ask(&mut duel, 0, vec![id(3)]);
    assert_eq!(super::veil_of(&duel, &group_of(&duel, id(2))), veil::PHASED);
}

/// A group is dimmed only when the question leaves out every member: one
/// member on offer lights the group.
#[test]
fn a_group_with_one_member_on_offer_is_not_dimmed() {
    let mut duel = table();
    ask(&mut duel, 0, vec![id(3)]);
    let mut group = group_of(&duel, id(1));
    group.members.push(id(3));
    assert_eq!(super::veil_of(&duel, &group), 0);
    group.members = vec![id(1), id(2)];
    assert_eq!(super::veil_of(&duel, &group), veil::DIM);
}

/// Another seat's question dims nothing at this seat's table.
#[test]
fn another_seats_question_dims_nothing() {
    let mut duel = table();
    ask(&mut duel, 1, vec![id(3)]);
    assert_eq!(super::veil_of(&duel, &group_of(&duel, id(1))), 0);
}

// ------------------------------------------------- summoning sickness

/// Four creatures on this seat's side (distinct names, so none piles): Alpha
/// and Gamma summoning sick, Phased summoning sick and phased out, Delta
/// awake.
fn sleepy_table() -> Duel {
    let sick = |slot, name: &str| {
        let mut t = token(slot, 0, name, 1, 1);
        t.summoning_sick = true;
        t
    };
    let mut phased = sick(2, "Phased");
    phased.status = ObjectStatus::PHASED_OUT;
    let view = ViewBuilder::new(2)
        .with_battlefield(
            0,
            [
                sick(1, "Alpha"),
                phased,
                sick(3, "Gamma"),
                token(4, 0, "Delta", 1, 1),
            ],
        )
        .build();
    let mut duel = Duel::default();
    duel.receive_view(view);
    rebuild_board(&mut duel);
    duel
}

/// With no question open a summoning-sick creature is asleep, and one that
/// is not is in no state.
#[test]
fn a_summoning_sick_creature_with_no_question_is_asleep() {
    let duel = sleepy_table();
    assert!(duel.interaction.is_none());
    let alpha = group_of(&duel, id(1));
    assert!(alpha.summoning_sick, "the view's flag reaches the group");
    assert_eq!(super::veil_of(&duel, &alpha), veil::ASLEEP);
    assert_eq!(super::veil_of(&duel, &group_of(&duel, id(4))), 0);
}

/// A sleeping creature the question leaves out is asleep and dimmed: the two
/// states are laid together.
#[test]
fn a_summoning_sick_creature_left_out_by_the_question_is_asleep_and_dimmed() {
    let mut duel = sleepy_table();
    ask(&mut duel, 0, vec![id(4)]);
    assert_eq!(
        super::veil_of(&duel, &group_of(&duel, id(1))),
        veil::ASLEEP | veil::DIM
    );
}

/// A sleeping creature the question offers is asleep only: being a choice
/// lifts the dimming, not the sleep.
#[test]
fn a_summoning_sick_creature_on_offer_is_asleep_only() {
    let mut duel = sleepy_table();
    ask(&mut duel, 0, vec![id(3)]);
    assert_eq!(super::veil_of(&duel, &group_of(&duel, id(3))), veil::ASLEEP);
}

/// A creature both summoning sick and phased out is phased and nothing else,
/// with no question and with one that leaves it out.
#[test]
fn a_summoning_sick_phased_out_creature_is_phased_only() {
    let mut duel = sleepy_table();
    let phased = group_of(&duel, id(2));
    assert!(phased.summoning_sick);
    assert_eq!(super::veil_of(&duel, &phased), veil::PHASED);
    ask(&mut duel, 0, vec![id(4)]);
    assert_eq!(super::veil_of(&duel, &group_of(&duel, id(2))), veil::PHASED);
}

// ---------------------------------------------------------- the shader

/// The shader's veil bits are the Rust ones: they travel in one packed
/// `u32` and a mismatch would draw one state as another, silently.
#[test]
fn the_shaders_veil_bits_are_the_rust_ones() {
    let src = include_str!("../shaders/card_common.wesl");
    let read = |name| cardmat::tests::wgsl_const(src, name) as u32;
    assert_eq!(read("VEIL_DIM"), veil::DIM);
    assert_eq!(read("VEIL_PHASED"), veil::PHASED);
    assert_eq!(read("VEIL_ASLEEP"), veil::ASLEEP);
}
