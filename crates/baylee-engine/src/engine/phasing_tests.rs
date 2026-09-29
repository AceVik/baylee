//! Phased-out permanents are treated as though they do not exist, except by
//! rules and effects that mention them (CR 702.26b) — #209.
//!
//! `GameState::battlefield_seen` (and `battlefield_view`, which collects it)
//! is the battlefield as the rules see it. A raw
//! `zones.list(ZoneLocation::Battlefield)` walk sees phased-out permanents,
//! and `eval::matches` never reads `PHASED_OUT`, so every raw walk is either
//! a defect or a deliberate exception. The lint below makes each one say
//! which: a raw walk carries a `// phasing:` comment giving its reason, or it
//! is still in [`UNAUDITED`], a table that only shrinks as walks are
//! audited. The lint reads that one spelling. Walks over every object
//! (state hashing, the projection refresh over `arena`, cleanup's damage
//! wipe, which CR 514.2 extends to phased-out permanents) are not
//! battlefield queries and are not counted.

use super::testkit::*;
use super::*;
use crate::object::Status;
use crate::zone::ZoneLocation;

fn drowned_catacomb() -> baylee_core::ids::CardIndex {
    card_index("819fc966-434e-470f-91e9-a38df974ad17")
}

fn blackcleave_cliffs() -> baylee_core::ids::CardIndex {
    card_index("5ad94412-6f79-4c5d-bbd4-4ef5779a7b6d")
}

fn swamp() -> baylee_core::ids::CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}

/// Plays `land` from hand on turn one over `board`, with the first
/// `phased` permanents of `board` phased out, and says whether it arrived
/// tapped.
fn arrives_tapped(
    board: &[baylee_core::ids::CardIndex],
    phased: usize,
    land: baylee_core::ids::CardIndex,
) -> bool {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(209, basic_forest())
        .battlefield(0, board)
        .hand(0, &[land])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let seated: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Battlefield).clone();
    let state = engine.dev_state_mut(p0).expect("the harness trusts itself");
    for id in seated.iter().take(phased) {
        state
            .object_mut(*id)
            .expect("seated")
            .status
            .insert(Status::PHASED_OUT);
    }
    engine.refresh_offer();
    let card = in_hand(&engine, p0, land).expect("in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card })
        .expect("a land drop on turn one");
    engine
        .state()
        .object(card)
        .expect("played")
        .status
        .contains(Status::TAPPED)
}

/// "This land enters tapped unless you control an Island or a Swamp." A
/// phased-out Swamp is not controlled by anybody the rules can see.
#[test]
fn a_phased_out_swamp_does_not_untap_a_checkland() {
    assert!(
        !arrives_tapped(&[swamp()], 0, drowned_catacomb()),
        "the control: a Swamp that is there lets the Catacomb enter untapped"
    );
    assert!(
        arrives_tapped(&[swamp()], 1, drowned_catacomb()),
        "a phased-out Swamp still satisfied the check"
    );
}

/// "This land enters tapped unless you control two or fewer other lands."
/// Three lands with one phased out is two.
#[test]
fn a_phased_out_land_does_not_count_against_a_fastland() {
    let three = [swamp(), swamp(), swamp()];
    assert!(
        arrives_tapped(&three, 0, blackcleave_cliffs()),
        "the control: three other lands make the Cliffs enter tapped"
    );
    assert!(
        !arrives_tapped(&three, 1, blackcleave_cliffs()),
        "a phased-out land was counted as one of the three"
    );
}

fn icy_manipulator() -> baylee_core::ids::CardIndex {
    card_index("3608f1f7-8dc5-4dd1-ae91-c830e1de9529")
}

fn glorious_anthem() -> baylee_core::ids::CardIndex {
    card_index("e3886fe8-9b76-4613-8891-4ec74657c087")
}

fn doubling_season() -> baylee_core::ids::CardIndex {
    card_index("01546b7d-a233-4176-8843-d732074dc5b6")
}

fn soul_warden() -> baylee_core::ids::CardIndex {
    card_index("f3fad295-1af2-4ecc-8546-b121ad6be27b")
}

/// Seat 0 in its first main phase of turn one over `board`, `hand` in hand.
fn seat_zero(
    board: &[baylee_core::ids::CardIndex],
    hand: &[baylee_core::ids::CardIndex],
) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(209, basic_forest())
        .battlefield(0, board)
        .hand(0, hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, PlayerId::new(0));
    engine
}

/// Resolves `effects` as seat 0's, `source` their source: the resolver runs,
/// as it does for every spell and ability, and asks nothing.
fn resolve_now(
    engine: &mut Engine<RegistryLookup>,
    source: ObjectId,
    effects: Vec<baylee_cards_dsl::Effect>,
) {
    let p0 = PlayerId::new(0);
    let state = engine.dev_state_mut(p0).expect("the harness trusts itself");
    let mut res = crate::resolve::Resolution {
        source,
        on_stack: source,
        controller: p0,
        effects,
        pc: 0,
        targets: smallvec::SmallVec::new(),
        second_targets: smallvec::SmallVec::new(),
        x: None,
        chosen_player: None,
        target_players: baylee_core::ids::SeatSet::new(),
        event_object: None,
        awaiting: None,
        targeted: false,
        mana_ability: false,
        countered_source: None,
        target_lki: None,
        retarget_left: None,
    };
    assert!(
        matches!(
            crate::resolve::run(state, &mut res),
            crate::resolve::Flow::Complete
        ),
        "the resolution asks nothing"
    );
    state.refresh_characteristics();
    engine.refresh_offer();
}

/// "~ phases out", resolved with `id` as its own source: the resolver
/// `Effect::PhaseOut` runs, not a status written by hand.
fn phase_out(engine: &mut Engine<RegistryLookup>, id: ObjectId) {
    resolve_now(
        engine,
        id,
        vec![baylee_cards_dsl::Effect::PhaseOut { target: None }],
    );
    assert!(is_phased_out(engine, id), "the resolver phased it out");
}

fn is_phased_out(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .is_some_and(|o| o.status.contains(Status::PHASED_OUT))
}

/// Everything registered from `source`'s static abilities, as (effect id,
/// timestamp) pairs.
fn statics_of(engine: &Engine<RegistryLookup>, source: ObjectId) -> Vec<(u32, u64)> {
    engine
        .state()
        .effects
        .iter()
        .filter(|fx| fx.source == Some(source) && fx.origin == crate::effects::EffectOrigin::Static)
        .map(|fx| (fx.id.get(), fx.timestamp))
        .collect()
}

/// A phased-out permanent "can't affect or be affected by anything else in
/// the game" (CR 702.26b), and activating an ability is its controller
/// using it: Icy Manipulator's ability and a Forest's mana are not offered
/// while they are phased out, and an activation sent anyway is refused.
#[test]
fn a_phased_out_permanent_offers_none_of_its_abilities() {
    let p0 = PlayerId::new(0);
    let forest = basic_forest();
    let mut engine = seat_zero(&[icy_manipulator(), forest, forest, forest], &[]);
    let icy = on_battlefield(&engine, p0, icy_manipulator()).expect("seated");
    let lands: Vec<ObjectId> = mine(&engine, p0, forest, crate::zone::Zone::Battlefield);
    let away = lands[0];
    // The offer lists an ability whose mana is already floating.
    tap_mana_where(&mut engine, p0, |id| id == lands[2]);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("seat 0 holds priority in its main phase");
    };
    assert!(
        legal.abilities.contains(&(icy, 0)) && legal.mana_abilities.contains(&away),
        "the control: phased in, both are offered"
    );

    phase_out(&mut engine, icy);
    phase_out(&mut engine, away);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("seat 0 still holds priority");
    };
    assert!(
        !legal.abilities.iter().any(|&(source, _)| source == icy),
        "a phased-out Icy Manipulator's ability was offered"
    );
    assert!(
        !legal.mana_abilities.contains(&away),
        "a phased-out Forest's mana was offered"
    );
    assert!(
        legal.mana_abilities.contains(&lands[1]),
        "the Forest still phased in is offered"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: icy,
                    ability_index: 0,
                },
            )
            .is_err(),
        "a phased-out Icy Manipulator was activated"
    );
    assert!(
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: away })
            .is_err(),
        "a phased-out Forest was tapped for mana"
    );
}

/// A static ability of a phased-out permanent does not apply, and neither
/// does its replacement effect (CR 702.26b). Phasing in is not entering
/// (CR 702.26d), so the same effect comes back with the timestamp it had.
#[test]
fn a_phased_out_permanents_statics_and_replacements_stop_until_it_phases_in() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = seat_zero(
        &[glorious_anthem(), doubling_season(), quiet_creature()],
        &[],
    );
    let anthem = on_battlefield(&engine, p0, glorious_anthem()).expect("seated");
    let season = on_battlefield(&engine, p0, doubling_season()).expect("seated");
    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("seated");
    let registered = statics_of(&engine, anthem);
    assert_eq!(pt(&engine, elf), (2, 2), "the control: the anthem applies");
    assert_eq!(registered.len(), 1, "the anthem registered its static");
    let seasons = |engine: &Engine<RegistryLookup>| {
        engine
            .state()
            .replacement_rules
            .iter()
            .filter(|r| r.source == season)
            .count()
    };
    let doubled = seasons(&engine);
    assert!(
        doubled > 0,
        "the control: Doubling Season registered its rules"
    );

    phase_out(&mut engine, anthem);
    phase_out(&mut engine, season);
    engine
        .apply(p0, PlayerAction::PassPriority)
        .expect("seat 0 passes");
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p1),
        "seat 1 holds priority in the same step, got {:?}",
        engine.pending()
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "a phased-out Glorious Anthem still pumps the Elf"
    );
    assert_eq!(
        statics_of(&engine, anthem),
        [],
        "a phased-out anthem's static is among the active effects"
    );
    assert_eq!(
        seasons(&engine),
        0,
        "a phased-out Doubling Season's replacement rules are registered"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        engine
            .state()
            .object(anthem)
            .is_some_and(|o| !o.status.contains(Status::PHASED_OUT)),
        "the anthem phased in at seat 0's untap step"
    );
    assert_eq!(pt(&engine, elf), (2, 2), "the anthem applies again");
    assert_eq!(
        statics_of(&engine, anthem),
        registered,
        "the anthem's effect came back as it was, timestamp and all"
    );
    assert_eq!(
        seasons(&engine),
        doubled,
        "Doubling Season's rules are back"
    );
}

/// A phased-out Soul Warden sees nothing enter (CR 702.26b): its trigger
/// is not collected while it is phased out.
#[test]
fn a_phased_out_permanents_triggers_do_not_trigger() {
    let p0 = PlayerId::new(0);
    let forest = basic_forest();
    let elf = quiet_creature();
    let mut engine = seat_zero(&[soul_warden(), forest, forest], &[elf, elf]);
    let warden = on_battlefield(&engine, p0, soul_warden()).expect("seated");
    let lands: Vec<ObjectId> = mine(&engine, p0, forest, crate::zone::Zone::Battlefield);
    let life = |engine: &Engine<RegistryLookup>| engine.state().players[0].life;
    let before = life(&engine);

    tap_mana_where(&mut engine, p0, |id| id == lands[0]);
    cast_with_floating(&mut engine, p0, elf);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        life(&engine),
        before + 1,
        "the control: Soul Warden sees the first Elf enter"
    );

    phase_out(&mut engine, warden);
    tap_mana_where(&mut engine, p0, |id| id == lands[1]);
    cast_with_floating(&mut engine, p0, elf);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        life(&engine),
        before + 1,
        "a phased-out Soul Warden saw the second Elf enter"
    );
}

fn holy_strength() -> baylee_core::ids::CardIndex {
    card_index("9357de36-f8be-4f49-b2c8-9fe9eaf82b07")
}

fn bonesplitter() -> baylee_core::ids::CardIndex {
    card_index("452e3f5f-ce17-4682-966b-5cc100210aee")
}

fn plains() -> baylee_core::ids::CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}

/// Seat 0's Elf wearing seat 0's Bonesplitter and seat 1's Holy Strength,
/// in seat 0's first main phase: `(engine, elf, bonesplitter, aura)`.
fn dressed_elf() -> (Engine<RegistryLookup>, ObjectId, ObjectId, ObjectId) {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(209, basic_forest())
        .battlefield(0, &[quiet_creature(), bonesplitter()])
        .battlefield(1, &[holy_strength()])
        .start();
    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("seated");
    let splitter = on_battlefield(&engine, p0, bonesplitter()).expect("seated");
    let aura = on_battlefield(&engine, p1, holy_strength()).expect("seated");
    {
        // What a starting battlefield cannot say, set before the first
        // state-based check would put the Aura into a graveyard.
        let state = engine.dev_state_mut(p0).expect("the harness trusts itself");
        for id in [splitter, aura] {
            state.object_mut(id).expect("seated").attached_to = Some(elf);
        }
        state.invalidate_projections();
    }
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    (engine, elf, splitter, aura)
}

/// "When a permanent phases out, any Auras, Equipment, or Fortifications
/// attached to that permanent phase out at the same time", and one that
/// phased out that way "won't phase in by itself, but instead phases in
/// along with the permanent it's attached to" (CR 702.26g). Nothing is
/// unattached on the way (CR 702.26d, 702.26j).
///
/// Seat 1's Holy Strength on seat 0's Elf is the half that tells "along
/// with" from "at its controller's untap step": seat 1's untap step comes
/// first and must leave it where it is.
#[test]
fn what_is_attached_to_a_permanent_phases_out_and_in_with_it() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let (mut engine, elf, splitter, aura) = dressed_elf();
    assert_eq!(
        pt(&engine, elf),
        (4, 3),
        "the control: the Elf is a 1/1 with +2/+0 and +1/+2"
    );

    phase_out(&mut engine, elf);
    assert!(
        is_phased_out(&engine, splitter),
        "the Bonesplitter stayed phased in when the Elf it equips phased out"
    );
    assert!(
        is_phased_out(&engine, aura),
        "the Holy Strength stayed phased in when the Elf it enchants phased out"
    );
    let attached = |engine: &Engine<RegistryLookup>, id| {
        engine
            .state()
            .object(id)
            .filter(|o| o.zone == crate::zone::Zone::Battlefield)
            .and_then(|o| o.attached_to)
    };
    for id in [splitter, aura] {
        assert_eq!(
            attached(&engine, id),
            Some(elf),
            "phasing out unattached it"
        );
    }

    reach_their_main_phase(&mut engine, p1);
    assert!(
        is_phased_out(&engine, aura),
        "seat 1's Aura phased in by itself at seat 1's untap step"
    );
    assert!(
        is_phased_out(&engine, elf) && is_phased_out(&engine, splitter),
        "the Elf and its Equipment phased in at seat 1's untap step"
    );

    reach_their_main_phase(&mut engine, p0);
    for (id, what) in [
        (elf, "Elf"),
        (splitter, "Bonesplitter"),
        (aura, "Holy Strength"),
    ] {
        assert!(
            !is_phased_out(&engine, id),
            "the {what} did not phase in at seat 0's untap step"
        );
    }
    for id in [splitter, aura] {
        assert_eq!(
            attached(&engine, id),
            Some(elf),
            "it phased in on the battlefield, attached to the Elf"
        );
    }
    assert_eq!(pt(&engine, elf), (4, 3), "both apply again");
}

/// Indirect phasing is transitive: an Aura on an Equipment on a creature
/// phases out with the creature, since it is attached to a permanent that
/// phases out (CR 702.26g), and phases in with it. The attachments are
/// written by hand; whether an Aura may enchant an Equipment is the
/// state-based actions' question, asked after this one.
#[test]
fn what_is_attached_to_an_attachment_phases_out_and_in_with_it_too() {
    let p0 = PlayerId::new(0);
    let (mut engine, elf, splitter, aura) = dressed_elf();
    let state = engine.dev_state_mut(p0).expect("the harness trusts itself");
    state.object_mut(aura).expect("seated").attached_to = Some(splitter);
    state.phase_out(&[elf]);
    let status = |state: &crate::state::GameState, id| {
        state.object(id).map(|o| {
            (
                o.status.contains(Status::PHASED_OUT),
                o.status.contains(Status::PHASED_OUT_INDIRECTLY),
            )
        })
    };
    assert_eq!(status(state, elf), Some((true, false)), "the Elf, directly");
    assert_eq!(
        status(state, splitter),
        Some((true, true)),
        "the Bonesplitter, with the Elf"
    );
    assert_eq!(
        status(state, aura),
        Some((true, true)),
        "the Aura on the Bonesplitter, with the Bonesplitter"
    );
    state.phase_in(elf);
    for id in [elf, splitter, aura] {
        assert_eq!(
            status(state, id),
            Some((false, false)),
            "all three are back"
        );
    }
}

/// "You control a phased-out creature. You cast a spell that says 'Destroy
/// all creatures.' The phased-out creature is not destroyed." (CR 702.26b's
/// own example.)
#[test]
fn destroy_all_creatures_passes_over_a_phased_out_one() {
    let p0 = PlayerId::new(0);
    let forest = basic_forest();
    let elf = quiet_creature();
    let engine = &mut seat_zero(&[forest, elf, elf], &[]);
    let elves = mine(engine, p0, elf, crate::zone::Zone::Battlefield);
    let land = on_battlefield(engine, p0, forest).expect("seated");
    phase_out(engine, elves[0]);

    resolve_now(
        engine,
        land,
        vec![baylee_cards_dsl::Effect::DestroyAll {
            filter: &baylee_cards_dsl::Filter::CREATURE,
            no_regen: false,
        }],
    );
    let zone = |engine: &Engine<RegistryLookup>, id| engine.state().object(id).map(|o| o.zone);
    assert_ne!(
        zone(engine, elves[1]),
        Some(crate::zone::Zone::Battlefield),
        "the control: the Elf phased in is destroyed"
    );
    assert_eq!(
        zone(engine, elves[0]),
        Some(crate::zone::Zone::Battlefield),
        "the phased-out Elf was destroyed"
    );
    assert!(
        is_phased_out(engine, elves[0]),
        "and it is still phased out"
    );
}

/// A continuous effect from a resolution that changes characteristics
/// leaves a phased-out permanent out of the set it affects (CR 702.26e), and
/// the set is fixed as the effect begins (CR 611.2c): a creature that phases
/// in later is not in it. The effect here lasts indefinitely, so the Elf is
/// read after it has phased in.
#[test]
fn an_effect_that_began_while_it_was_phased_out_never_reaches_it() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let forest = basic_forest();
    let elf = quiet_creature();
    let engine = &mut seat_zero(&[forest, elf, elf], &[]);
    let elves = mine(engine, p0, elf, crate::zone::Zone::Battlefield);
    let land = on_battlefield(engine, p0, forest).expect("seated");
    phase_out(engine, elves[0]);

    resolve_now(
        engine,
        land,
        vec![baylee_cards_dsl::Effect::CreateContinuousEffect {
            layer: baylee_cards_dsl::Layer::PtModify,
            filter: &baylee_cards_dsl::Filter::CREATURE,
            modifier: baylee_cards_dsl::Modifier::ModifyPT(1, 1),
            duration: baylee_cards_dsl::Duration::Indefinitely,
        }],
    );
    assert_eq!(
        pt(engine, elves[1]),
        (2, 2),
        "the control: the Elf phased in gets +1/+1"
    );
    reach_their_main_phase(engine, p1);
    reach_their_main_phase(engine, p0);
    assert!(
        !is_phased_out(engine, elves[0]),
        "the Elf phased in at seat 0's untap step"
    );
    assert_eq!(
        pt(engine, elves[0]),
        (1, 1),
        "an effect that fixed its set while the Elf was phased out pumps it"
    );
    assert_eq!(pt(engine, elves[1]), (2, 2), "and the other Elf keeps it");
}

/// A phased-out permanent "can't affect or be affected by anything else in
/// the game" (CR 702.26b): Glorious Anthem arriving while the Elf is phased
/// out pumps the Elf beside it and not the Elf that is not there, until that
/// one phases in (CR 702.26c).
#[test]
fn an_anthem_that_arrives_while_it_is_phased_out_waits_for_it_to_phase_in() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let forest = basic_forest();
    let elf = quiet_creature();
    let engine = &mut seat_zero(
        &[plains(), plains(), forest, elf, elf],
        &[glorious_anthem()],
    );
    let elves = mine(engine, p0, elf, crate::zone::Zone::Battlefield);
    phase_out(engine, elves[0]);
    tap_mana_where(engine, p0, |id| !elves.contains(&id));
    cast_with_floating(engine, p0, glorious_anthem());
    pass_until(engine, stack_is_empty);
    assert!(
        on_battlefield(engine, p0, glorious_anthem()).is_some(),
        "the control: the anthem resolved"
    );
    assert_eq!(
        pt(engine, elves[1]),
        (2, 2),
        "the control: it pumps the Elf phased in"
    );
    assert_eq!(
        pt(engine, elves[0]),
        (1, 1),
        "an anthem that arrived while the Elf was phased out pumps it"
    );

    reach_their_main_phase(engine, p1);
    reach_their_main_phase(engine, p0);
    assert!(!is_phased_out(engine, elves[0]), "the Elf phased in");
    assert_eq!(
        pt(engine, elves[0]),
        (2, 2),
        "and the anthem applies to it now"
    );
}

/// "All phased-out permanents that the active player controlled when they
/// phased out phase in" (CR 502.1). Seat 1 takes seat 0's Elf until end of
/// turn and phases it out; the theft ends at seat 1's cleanup, while the
/// Elf is phased out (CR 702.26f), but the Elf phased out under seat 1's
/// control, so it phases in at seat 1's untap step and not at seat 0's. It
/// is seat 0's again once it is back.
#[test]
fn a_permanent_phases_in_under_the_player_it_phased_out_under() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let engine = &mut seat_zero(&[quiet_creature()], &[]);
    let elf = on_battlefield(engine, p0, quiet_creature()).expect("seated");
    reach_their_main_phase(engine, p1);
    {
        let state = engine.dev_state_mut(p1).expect("the harness trusts itself");
        let filter = crate::effects::EffectFilter::object(state, elf);
        let timestamp = state.next_timestamp();
        state.effects.register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: None,
            controller: p1,
            origin: crate::effects::EffectOrigin::Resolution,
            layer: baylee_cards_dsl::Layer::Control,
            timestamp,
            duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
            filter,
            modifier: baylee_cards_dsl::Modifier::GainControl,
        });
        state.refresh_characteristics();
    }
    engine.refresh_offer();
    let controller =
        |engine: &Engine<RegistryLookup>| engine.state().object(elf).map(|o| o.controller);
    assert_eq!(
        controller(engine),
        Some(p1),
        "the control: seat 1 took the Elf"
    );

    phase_out(engine, elf);
    reach_their_main_phase(engine, p0);
    assert!(
        is_phased_out(engine, elf),
        "the Elf phased in at seat 0's untap step, but it phased out under seat 1's control"
    );

    reach_their_main_phase(engine, p1);
    assert!(
        !is_phased_out(engine, elf),
        "the Elf did not phase in at seat 1's untap step"
    );
    assert_eq!(
        controller(engine),
        Some(p0),
        "the theft ended while the Elf was phased out; it is seat 0's again"
    );
}

fn mountain() -> baylee_core::ids::CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

/// `{3}{R}{R}` "Gain control of target creature until end of turn. Untap
/// that creature. It gains haste until end of turn": the pool's Threaten.
fn song_mad_treachery() -> baylee_core::ids::CardIndex {
    card_index("81b61770-2ed5-4a50-84d0-97790002fc5a")
}

/// A player who leaves takes with them the effects that gave them control
/// of something, and what they still control is exiled (CR 800.4a). A
/// permanent that phased out under them and is not theirs by default is
/// not among it: it "phases in during the next untap step after that
/// player's next turn would have begun" (CR 702.26n). Three seats, so the
/// game outlives the leaver: seat 0 takes seat 1's Elf until end of turn,
/// phases it out and concedes. The Elf is seat 1's again, still on the
/// battlefield, and back by seat 1's second turn at the latest.
#[test]
fn a_permanent_phased_out_under_a_player_who_leaves_stays_and_comes_back() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let engine = &mut Duel::table(281, quiet_creature(), 3)
        .battlefield(0, &[mountain(); 5])
        .battlefield(1, &[quiet_creature()])
        .hand(0, &[song_mad_treachery()])
        .start();
    keep_mulligans(engine);
    assert!(
        walk_to_own_main(engine, p0),
        "seat 0 reaches its main phase"
    );
    let elf = on_battlefield(engine, p1, quiet_creature()).expect("seat 1's Elf");
    tap_all_mana(engine, p0);
    cast_with_floating(engine, p0, song_mad_treachery());
    if let Pending::ChooseCastMode { options, .. } = engine.pending().clone() {
        let slot = options
            .iter()
            .position(|o| matches!(o.kind, crate::choice::CastModeKind::Face(0)))
            .expect("the front face is one of the ways to play it");
        engine.apply(p0, PlayerAction::ChooseMode(slot)).unwrap();
    }
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf is a target");
    pass_until(engine, stack_is_empty);
    let controller =
        |engine: &Engine<RegistryLookup>| engine.state().object(elf).map(|o| o.controller);
    assert_eq!(
        controller(engine),
        Some(p0),
        "the control: seat 0 took the Elf"
    );

    phase_out(engine, elf);
    engine
        .apply(p0, PlayerAction::Concede)
        .expect("seat 0 concedes");
    assert_eq!(
        engine.state().object(elf).map(|o| o.zone),
        Some(crate::zone::Zone::Battlefield),
        "the Elf was exiled with what seat 0 controlled, but seat 0's control ended as it left"
    );
    assert!(is_phased_out(engine, elf), "and it is still phased out");
    assert_eq!(controller(engine), Some(p1), "it is seat 1's again");

    reach_their_main_phase(engine, p1);
    if is_phased_out(engine, elf) {
        reach_their_main_phase(engine, PlayerId::new(2));
        reach_their_main_phase(engine, p1);
    }
    assert!(!is_phased_out(engine, elf), "the Elf never phased in again");
    assert_eq!(controller(engine), Some(p1), "seat 1's");
}

/// A phased-out permanent is not offered as a target (CR 702.26b; the
/// target menu reads `battlefield_view`). This held before indirect phasing
/// and the walk audit and is pinned here beside them: Icy Manipulator may
/// tap the Elf phased in and not the one phased out.
#[test]
fn a_phased_out_permanent_is_not_offered_as_a_target() {
    let p0 = PlayerId::new(0);
    let forest = basic_forest();
    let elf = quiet_creature();
    let engine = &mut seat_zero(&[icy_manipulator(), forest, elf, elf], &[]);
    let icy = on_battlefield(engine, p0, icy_manipulator()).expect("seated");
    let elves = mine(engine, p0, elf, crate::zone::Zone::Battlefield);
    phase_out(engine, elves[0]);
    let land = on_battlefield(engine, p0, forest).expect("seated");
    tap_mana_where(engine, p0, |id| id == land);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: icy,
                ability_index: 0,
            },
        )
        .expect("Icy Manipulator is offered");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("Icy asks for its target, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&elves[1]),
        "the control: the Elf phased in is a target"
    );
    assert!(
        !options.contains(&elves[0]),
        "the phased-out Elf was offered as a target"
    );
}

/// Raw battlefield walks with no `// phasing:` reason yet, per file, relative
/// to `crates/baylee-engine/src`. Equality, not a ceiling: auditing a walk
/// (switching it to `battlefield_seen`, or giving it a reason) makes this
/// test fail until the row is lowered, so the table cannot go stale in the
/// direction that hides work.
///
/// Empty since 2026-09-29: of the 34 walks it listed on 2026-09-24, 33 now
/// call `battlefield_seen` or `battlefield_view` and the untap step's
/// phase-in gives its reason; two walks new that day (the deathtouch reset
/// after the lethal-damage check, and `GameState::phase_in`) give theirs. A
/// new walk without one comes back here as a row.
const UNAUDITED: &[(&str, usize)] = &[];

/// Where the lint looks: the engine's own source, tests excluded. Any
/// directory named `*_tests` is test code, not the two there were: a list of
/// them read `mechanics_tests/` as engine source when it arrived.
fn is_test_source(rel: &str) -> bool {
    rel.ends_with("_tests.rs")
        || rel
            .split('/')
            .rev()
            .skip(1)
            .any(|dir| dir.ends_with("_tests"))
        || matches!(
            rel,
            "engine/testkit.rs" | "engine/synthetic.rs" | "engine/tests.rs"
        )
}

fn sources(dir: &std::path::Path, root: &std::path::Path, out: &mut Vec<(String, String)>) {
    for entry in std::fs::read_dir(dir).expect("engine source directory") {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            sources(&path, root, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let rel = path
                .strip_prefix(root)
                .expect("under the root")
                .to_string_lossy()
                .replace('\\', "/");
            if !is_test_source(&rel) {
                let text = std::fs::read_to_string(&path).expect("readable source");
                out.push((rel, text));
            }
        }
    }
}

/// Per file: raw walks, and how many of them give no reason.
fn walks(text: &str) -> (usize, usize) {
    // An inline test module is test code too, and closes its file by
    // convention; a `#[cfg(test)] mod x;` declaration is not one.
    let all: Vec<&str> = text.lines().collect();
    let cut = all
        .windows(2)
        .position(|w| {
            w[0].trim() == "#[cfg(test)]"
                && w[1].starts_with("mod ")
                && w[1].trim_end().ends_with('{')
        })
        .unwrap_or(all.len());
    let lines = &all[..cut];
    let mut raw = 0;
    let mut unexplained = 0;
    for (i, line) in lines.iter().enumerate() {
        let comment = line.trim_start().starts_with("//");
        if !comment && line.contains(".list(") && line.contains("ZoneLocation::Battlefield)") {
            raw += 1;
            let reasoned = lines[i.saturating_sub(3)..=i]
                .iter()
                .any(|l| l.contains("// phasing:"));
            if !reasoned {
                unexplained += 1;
            }
        }
    }
    (raw, unexplained)
}

/// Every raw battlefield walk in the engine says why it may see phased-out
/// permanents, or is still on the list of walks nobody has audited.
#[test]
fn every_raw_battlefield_walk_gives_its_phasing_reason() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    sources(&root, &root, &mut files);
    let mut total = 0;
    let mut found: Vec<(String, usize)> = Vec::new();
    for (rel, text) in &files {
        let (raw, unexplained) = walks(text);
        total += raw;
        if unexplained > 0 {
            found.push((rel.clone(), unexplained));
        }
    }
    found.sort();
    // A floor and a ceiling on what was read: a walk that found no files,
    // or matched something it should not, would otherwise pass as clean.
    assert!(
        (30..=120).contains(&files.len()),
        "read {} engine source files, measured 36 on 2026-09-24",
        files.len()
    );
    assert!(
        (5..=200).contains(&total),
        "found {total} raw battlefield walks, measured 7 on 2026-09-29"
    );
    let want: Vec<(String, usize)> = UNAUDITED
        .iter()
        .map(|(file, n)| ((*file).to_string(), *n))
        .collect();
    assert_eq!(
        found,
        want,
        "raw battlefield walks without a `// phasing:` reason changed. A new \
         walk wants `GameState::battlefield_seen` or a reason; an audited one \
         wants its row lowered. Today's table:\n{}",
        found
            .iter()
            .map(|(file, n)| format!("    (\"{file}\", {n}),"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// The lint's counter-test: it reads both halves of a walk.
#[test]
fn the_lint_tells_a_reasoned_walk_from_a_bare_one() {
    let bare = "fn f() {\n    s.zones\n        .list(ZoneLocation::Battlefield)\n}\n";
    let reasoned = "fn f() {\n    // phasing: the untap step phases them in.\n    s.zones\n        .list(ZoneLocation::Battlefield)\n}\n";
    let tested =
        "fn f() {}\n#[cfg(test)]\nmod tests {\n    s.zones.list(ZoneLocation::Battlefield);\n}\n";
    let declared =
        "#[cfg(test)]\nmod x_tests;\nfn f() {\n    s.zones.list(ZoneLocation::Battlefield)\n}\n";
    let quoted =
        "/// Unlike `zones.list(ZoneLocation::Battlefield)`, this skips them.\nfn f() {}\n";
    assert_eq!(walks(bare), (1, 1));
    assert_eq!(walks(reasoned), (1, 0));
    assert_eq!(walks(tested), (0, 0));
    assert_eq!(walks(declared), (1, 1), "a module declaration is not a cut");
    assert_eq!(walks(quoted), (0, 0), "a comment quoting a walk is not one");
}
