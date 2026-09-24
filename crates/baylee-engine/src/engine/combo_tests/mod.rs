//! Cards played *together*: one card changes what another is legally
//! offered.
//!
//! A single-card test asks whether a card's own rules text works. This
//! module asks the question a game actually poses — whether two cards that
//! have never met produce the answer the rules give — and it asks it through
//! `LegalActions` and `Pending`, because that is the only place a client can
//! see an answer. Every test here carries a **bystander**: a permanent that
//! must *not* be offered. An interaction that reaches one permanent too many
//! looks identical to a working one when the board holds a single candidate,
//! which is how a filter that plated every permanent in the game sat in the
//! pool unnoticed.
//!
//! The bystander is usually across the table, because the two ways a scope
//! goes wrong are symmetrical: an effect that should stay on your own side
//! reaching an opponent's board, and one aimed at an opponent coming back to
//! your own.

mod copied_abilities;
mod doubling;
mod filters;
mod grants;
mod printed_faces;
mod spell_copies;
mod temporary_copies;
mod token_arrivals;
mod walkers;

use super::testkit::*;

use super::*;

use crate::choice::YesNoPrompt;

fn island() -> baylee_core::ids::CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}

fn plains() -> baylee_core::ids::CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}

fn swamp() -> baylee_core::ids::CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}

fn forest() -> baylee_core::ids::CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

fn riptide_laboratory() -> baylee_core::ids::CardIndex {
    card_index("444d50dd-a44a-42db-bbf6-d0978e3bd6a3")
}

fn maskwood_nexus() -> baylee_core::ids::CardIndex {
    card_index("9b2cdbed-c733-409b-b0e4-2c8960c25111")
}

fn snapcaster_mage() -> baylee_core::ids::CardIndex {
    card_index("2bb2eda7-3b38-4c56-870f-c3218a1056f5")
}

fn llanowar_elves() -> baylee_core::ids::CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
}

fn liquimetal_coating() -> baylee_core::ids::CardIndex {
    card_index("f4bdc551-c2eb-4a34-a3e3-b4a017c925af")
}

fn fracture() -> baylee_core::ids::CardIndex {
    card_index("f21d0319-0509-4ac1-b6e3-10955a26fd7a")
}

fn glasspool_mimic() -> baylee_core::ids::CardIndex {
    card_index("c178953c-3888-4edd-9d0c-265bd82b1d24")
}

fn earth_king_s_lieutenant() -> baylee_core::ids::CardIndex {
    card_index("9da9248d-1201-447f-b6c2-2b64af4f71c4")
}

fn ondu_cleric() -> baylee_core::ids::CardIndex {
    card_index("f4232466-dd6a-49bf-be6c-95905c3ded17")
}

fn werefox_bodyguard() -> baylee_core::ids::CardIndex {
    card_index("d5ee2ced-29f4-430f-962e-2f930b92624c")
}

fn karn_the_great_creator() -> baylee_core::ids::CardIndex {
    card_index("a20dd48d-d344-4db1-b0e9-a2b71c3cc9d1")
}

/// An artifact *land*, so the ability Karn's lock has to stop is a mana
/// ability — the case that says the lock reads CR 605.1 rather than
/// treating a mana ability as something other than an activated one.
fn vault_of_whispers() -> baylee_core::ids::CardIndex {
    card_index("09496421-74e4-466a-9546-56f2a0c8eef4")
}

fn skyclave_apparition() -> baylee_core::ids::CardIndex {
    card_index("d90af00a-d322-4265-9954-7b1e80702e18")
}

/// Mana value five, and nothing else about it matters: it is here to be
/// one over Skyclave Apparition's limit.
fn sea_gate_loremaster() -> baylee_core::ids::CardIndex {
    card_index("6eed122b-9760-47fd-8ba2-adeda8054e0d")
}

fn mycosynth_lattice() -> baylee_core::ids::CardIndex {
    card_index("ae1f2ab5-c6a5-4d49-a746-3cb4668bf805")
}

fn chromatic_lantern() -> baylee_core::ids::CardIndex {
    card_index("539f5396-d99a-417d-a84c-dff7930b5900")
}

fn privileged_position() -> baylee_core::ids::CardIndex {
    card_index("abd62af0-c17d-4f62-af15-9ea83037b990")
}

fn lightning_greaves() -> baylee_core::ids::CardIndex {
    card_index("ca204b66-8d0c-431a-8d34-282f7c2d17da")
}

fn vindicate() -> baylee_core::ids::CardIndex {
    card_index("63c1ac21-e3d8-40c2-8c09-3f31c52992ef")
}

fn doubling_season() -> baylee_core::ids::CardIndex {
    card_index("01546b7d-a233-4176-8843-d732074dc5b6")
}

fn panharmonicon() -> baylee_core::ids::CardIndex {
    card_index("76678885-3674-443d-b9a2-2a460cf6aac0")
}

fn darksteel_forge() -> baylee_core::ids::CardIndex {
    card_index("9b3bec05-441f-4fdf-8b51-69fa8613fcd4")
}

fn crib_swap() -> baylee_core::ids::CardIndex {
    card_index("2987c385-011a-4032-a516-a46d1e9dc9e8")
}

fn rite_of_replication() -> baylee_core::ids::CardIndex {
    card_index("fb60739e-1dc3-481d-a056-ad72e665c680")
}

fn thief_of_blood() -> baylee_core::ids::CardIndex {
    card_index("97d61346-bd53-4eb8-a920-6ae0382eb20d")
}

fn spark_double() -> baylee_core::ids::CardIndex {
    card_index("8dcb35e5-ae44-455f-86e3-4a77d496ff34")
}

fn sakashima_of_a_thousand_faces() -> baylee_core::ids::CardIndex {
    card_index("8ecdaf4b-4442-42da-9714-4257a83faf50")
}

fn padeem_consul_of_innovation() -> baylee_core::ids::CardIndex {
    card_index("0c7ba712-6a99-4d2f-9242-a2163a11f69c")
}

/// The one *non-mana* activated ability `source` is offering right now.
///
/// `LegalActions::abilities` carries a permanent's mana abilities too —
/// `mana_abilities` is the narrower list of lands that tap for their basic
/// type (CR 305.6), and a land with a printed `{T}: Add {C}` is offered
/// through the general list like anything else. A test that took the first
/// entry would activate Riptide Laboratory's mana ability and then wonder
/// why nothing asked it for a target.
#[track_caller]
fn offered_ability(
    engine: &Engine<RegistryLookup>,
    source: baylee_core::ids::ObjectId,
) -> Option<u32> {
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let abilities = engine
        .state()
        .object(source)
        .expect("the source is on the battlefield")
        .abilities(&RegistryLookup);
    legal.abilities.iter().find_map(|(id, index)| {
        (*id == source
            && !matches!(
                abilities.get(*index as usize),
                Some(AbilityDef::Activated {
                    mana_ability: true,
                    ..
                })
            ))
        .then_some(*index)
    })
}

/// Whether `LegalActions` is offering *any* ability of `source` right now,
/// a mana ability included.
///
/// [`offered_ability`] deliberately steps over mana abilities, because most
/// tests here are about the one ability a permanent has that is not one.
/// Karn's lock is the opposite question — it stops every activated ability
/// of an artifact, and a mana ability is one (CR 605.1).
#[track_caller]
fn offers_an_ability(engine: &Engine<RegistryLookup>, source: baylee_core::ids::ObjectId) -> bool {
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    legal.abilities.iter().any(|(id, _)| *id == source) || legal.mana_abilities.contains(&source)
}

/// The options a target choice is offering right now.
#[track_caller]
fn target_options(engine: &Engine<RegistryLookup>) -> Vec<baylee_core::ids::ObjectId> {
    match engine.pending() {
        Pending::ChooseTargets { options, .. } => options.clone(),
        other => panic!("expected a target choice, got {other:?}"),
    }
}

/// Both seats holding Vindicate ("Destroy target permanent"), with a
/// Privileged Position and an Elf on your side and a Wizard on theirs.
///
/// Vindicate is the removal to ask with because it targets *any* permanent:
/// the options it offers are the whole table, so what is missing from them
/// is a statement about the grant and not about the spell's own filter.
fn a_table_under_a_privileged_position() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(80, forest())
        .battlefield(
            0,
            &[
                privileged_position(),
                llanowar_elves(),
                plains(),
                swamp(),
                forest(),
            ],
        )
        .hand(0, &[vindicate()])
        .battlefield(1, &[snapcaster_mage(), plains(), swamp(), forest()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    engine
}

/// Taps everything that taps for mana for `seat` and casts `card` from its
/// hand, leaving the engine on the spell's target choice.
#[track_caller]
fn cast_from_hand(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    card: baylee_core::ids::CardIndex,
) {
    let spell = in_hand(engine, seat, card).expect("the spell is in hand");
    tap_all_mana(engine, seat);
    engine
        .apply(seat, PlayerAction::CastSpell { card: spell })
        .unwrap();
}

/// How many tokens `seat` controls.
///
/// Counted by `token` rather than by "a permanent the test did not seed": a
/// token is the one permanent that carries its definition instead of a
/// card, which is exactly the object Doubling Season's first sentence is
/// about.
#[must_use]
fn tokens_controlled(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            engine
                .state()
                .object(**id)
                .is_some_and(|o| o.controller == seat && o.token.is_some())
        })
        .count()
}

/// Taps everything `seat` has and activates the one non-mana ability
/// `source` is offering, leaving it on the stack.
#[track_caller]
fn activate(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    source: baylee_core::ids::ObjectId,
) {
    tap_mana_except(engine, seat, source);
    let index = offered_ability(engine, source).expect("the permanent is offering its ability");
    engine
        .apply(
            seat,
            PlayerAction::ActivateAbility {
                source,
                ability_index: index,
            },
        )
        .unwrap();
}

/// Darksteel Forge ("**Artifacts you control** have indestructible"), a
/// Liquimetal Coating on each side and an Elf beside the Forge, with a
/// Vindicate ("Destroy target permanent") in each hand.
///
/// Indestructible is not hexproof, which is why every test below reads the
/// battlefield rather than the options list: the permanent is still a legal
/// target and the spell still resolves — what it fails to do is destroy it
/// (CR 702.12b).
fn a_table_under_a_darksteel_forge(seed: u64) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(seed, forest())
        .battlefield(
            0,
            &[
                darksteel_forge(),
                liquimetal_coating(),
                llanowar_elves(),
                plains(),
                swamp(),
                forest(),
            ],
        )
        .hand(0, &[vindicate()])
        .battlefield(1, &[liquimetal_coating(), plains(), swamp(), forest()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    engine
}

/// Resolves `seat`'s Vindicate onto `victim` and returns whether it is still
/// on the battlefield afterwards.
#[track_caller]
fn vindicated(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    victim: baylee_core::ids::ObjectId,
) -> bool {
    cast_from_hand(engine, seat, vindicate());
    let options = target_options(engine);
    assert!(
        options.contains(&victim),
        "indestructible does not stop targeting, so the spell has to be \
         allowed to point at it before its failure means anything: \
         {options:?}"
    );
    engine
        .apply(
            seat,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .unwrap();
    pass_until(engine, stack_is_empty);
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .contains(&victim)
}

/// Permanents `seat` controls that no card stands behind.
///
/// [`tokens_controlled`] reads `token`, which is the token *definition* a
/// Treasure or a Shapeshifter was stamped out of, and a token copy of a
/// creature has none: `CreateTokenCopyOf` builds its object out of the
/// copied characteristics and stamps no definition on it. So the engine has
/// two notions of "this is a token" and the copy branches only satisfy the
/// second — the absence of a card behind the permanent — which is what this
/// counts.
#[must_use]
fn cardless_permanents(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            engine
                .state()
                .object(**id)
                .is_some_and(|o| o.controller == seat && o.card.is_none())
        })
        .count()
}

/// A duel where seat 0 holds `spell` and seat 1 has a creature to point it
/// at, with a Doubling Season on the seat `season` names.
///
/// Moving one enchantment across the table is the only difference between
/// the two tests that use each table, which is what makes the pair of
/// numbers mean anything: a rule read off the wrong seat gives the same
/// answer on a board that has only one Doubling Season on it.
fn a_table_with_a_season_on_one_side(
    seed: u64,
    season: usize,
    lands: &[baylee_core::ids::CardIndex],
    spell: baylee_core::ids::CardIndex,
) -> Engine<RegistryLookup> {
    let mut mine = lands.to_vec();
    let mut theirs = vec![llanowar_elves(), forest()];
    if season == 0 {
        mine.push(doubling_season());
    } else {
        theirs.push(doubling_season());
    }
    let mut engine = Duel::new(seed, forest())
        .battlefield(0, &mine)
        .hand(0, &[spell])
        .battlefield(1, &theirs)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, PlayerId::new(0));
    engine
}

/// Casts seat 0's spell at seat 1's Llanowar Elves and lets it resolve.
///
/// The wizard asks its questions in its own order and the spell is not on
/// the stack until the last of them is answered, so the loop answers
/// whatever is in front of it rather than assuming a sequence. Rite of
/// Replication is what made that necessary: its kicker is asked *after* the
/// target choice, and a test that passed priority in between found an empty
/// stack, called the spell resolved and counted a board nothing had
/// happened to yet.
#[track_caller]
fn aim_at_their_elf(
    engine: &mut Engine<RegistryLookup>,
    spell: baylee_core::ids::CardIndex,
    pay_extra: bool,
) -> baylee_core::ids::ObjectId {
    aim_at_theirs(engine, spell, llanowar_elves(), pay_extra)
}

/// The same, at whichever of seat 1's permanents `victim` names.
#[track_caller]
fn aim_at_theirs(
    engine: &mut Engine<RegistryLookup>,
    spell: baylee_core::ids::CardIndex,
    victim: baylee_core::ids::CardIndex,
    pay_extra: bool,
) -> baylee_core::ids::ObjectId {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let victim = on_battlefield(engine, p1, victim).expect("their permanent");
    cast_from_hand(engine, p0, spell);
    let mut aimed = false;
    loop {
        match engine.pending() {
            // `pay_extra` answers every optional additional cost the same
            // way, which is all these tests need: the only spell here that
            // has one is Rite of Replication, and its kicker is the whole
            // question in the test that pays it.
            Pending::YesNo { .. } => {
                engine.apply(p0, PlayerAction::YesNo(pay_extra)).unwrap();
            }
            Pending::ChooseTargets { .. } => {
                let options = target_options(engine);
                assert!(
                    options.contains(&victim),
                    "their creature is a legal target"
                );
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![victim],
                        },
                    )
                    .unwrap();
                aimed = true;
            }
            _ => break,
        }
    }
    assert!(aimed, "the spell asked for its target");
    pass_until(engine, stack_is_empty);
    victim
}

/// The one card-less permanent seat `seat` controls.
///
/// Asserting there is exactly one is the point: a test that took the first
/// of several would pass while the rest were inert.
#[track_caller]
fn the_copy_on(engine: &Engine<RegistryLookup>, seat: PlayerId) -> baylee_core::ids::ObjectId {
    let mut found: Vec<baylee_core::ids::ObjectId> = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.controller == seat && o.card.is_none())
        })
        .collect();
    assert_eq!(found.len(), 1, "exactly one token copy was created");
    found.pop().expect("the copy")
}

fn nesting_dovehawk() -> baylee_core::ids::CardIndex {
    card_index("fe8fc442-ed17-40b2-8624-69f2eed3f9be")
}

/// The `+1/+1` counters on the Nesting Dovehawk seat `seat` controls.
///
/// Both seats have one in the tests below, and the pair is the whole
/// measurement: "a creature token you control enters" is two claims, and a
/// scan that read the wrong seat's control would still put a counter
/// somewhere.
#[track_caller]
fn counters_on_the_dovehawk(engine: &Engine<RegistryLookup>, seat: PlayerId) -> u16 {
    plus_one_counters(engine, seat, nesting_dovehawk())
}

/// The `+1/+1` counters on the one permanent of `card` that seat `seat`
/// controls, which is how a trigger that places exactly one per firing is
/// counted.
#[track_caller]
fn plus_one_counters(
    engine: &Engine<RegistryLookup>,
    seat: PlayerId,
    card: baylee_core::ids::CardIndex,
) -> u16 {
    let obj = on_battlefield(engine, seat, card).expect("that seat's permanent");
    engine
        .state()
        .object(obj)
        .expect("a permanent on the battlefield is an object")
        .counters
        .get(baylee_cards_dsl::CounterKind::P1P1)
}

fn baleful_strix() -> baylee_core::ids::CardIndex {
    card_index("37688720-03de-4eca-a82d-a0afe8d58adc")
}

/// How many cards are left in seat `seat`'s library.
///
/// The measurement a draw is read off here, rather than the hand: the spell
/// doing the copying leaves the hand on its way to the stack, so a hand
/// counted before and after would be back where it started and would say
/// the same thing whether or not anything was drawn.
fn library_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Library(seat))
        .len()
}

fn great_divide_guide() -> baylee_core::ids::CardIndex {
    card_index("79e69a91-d580-47fb-be76-1e32c50d2fa0")
}

fn katara_the_fearless() -> baylee_core::ids::CardIndex {
    card_index("0972d46e-423b-454e-87c7-a2d40fb6fb6d")
}

fn karmic_guide() -> baylee_core::ids::CardIndex {
    card_index("8c31fec9-e4b3-4761-990e-7be38eb05604")
}

fn cursed_mirror() -> baylee_core::ids::CardIndex {
    card_index("4d67e2a7-4aa7-44cc-853b-500d7aac046d")
}

fn mountain() -> baylee_core::ids::CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

/// The **third** door a copy comes through, and the one that is not a new
/// object at all: `CopyOnEnterUntilEot` (Cursed Mirror, "you may have it
/// become a copy of any creature on the battlefield until end of turn,
/// except it has haste"), which is a layer-1 continuous effect rather than
/// a rewritten base, because it has to end.
///
/// Leaves the Mirror untapped and priority with seat 0, so what each caller
/// measures is the Mirror's own next activation.
fn a_mirror_that_became_their_elf(
    seed: u64,
) -> (Engine<RegistryLookup>, baylee_core::ids::ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(seed, forest())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[cursed_mirror()])
        .battlefield(1, &[llanowar_elves(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf");

    cast_from_hand(&mut engine, p0, cursed_mirror());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    assert!(
        target_options(&engine).contains(&elf),
        "\"any creature on the battlefield\" reaches across the table"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let mirror = on_battlefield(&engine, p0, cursed_mirror()).expect("the Mirror arrived");
    assert_eq!(
        pt(&engine, mirror),
        (1, 1),
        "the body came across, which is the half that already worked"
    );
    (engine, mirror)
}

/// The ability index the Mirror is offering, whatever it currently is.
fn the_mirrors_ability(engine: &Engine<RegistryLookup>, mirror: baylee_core::ids::ObjectId) -> u32 {
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("a main phase hands priority back");
    };
    legal
        .abilities
        .iter()
        .find_map(|(source, index)| (*source == mirror).then_some(*index))
        .expect("the Mirror is offering an ability")
}

/// Taps only the lands of one printing that `seat` controls.
///
/// [`tap_mana_except`] keeps a single source back; this keeps a whole
/// printing, which is what a test casting two spells in one main phase
/// needs — `cast_from_hand` taps everything, so the second spell finds an
/// empty board.
#[track_caller]
fn tap_only(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    card: baylee_core::ids::CardIndex,
) {
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        let is_it = engine
            .state()
            .object(source)
            .is_some_and(|o| o.card.is_some_and(|c| c.index == card));
        if is_it {
            engine
                .apply(seat, PlayerAction::ActivateManaAbility { source })
                .unwrap();
        }
    }
}

fn harabaz_druid() -> baylee_core::ids::CardIndex {
    card_index("ead985ec-f29f-4a3b-b8b1-061142cc5bd1")
}

fn esper_sentinel() -> baylee_core::ids::CardIndex {
    card_index("5def9f38-0a0b-4e8d-9f9d-29dcb46520b4")
}

fn sword_of_hearth_and_home() -> baylee_core::ids::CardIndex {
    card_index("913e6182-706a-4872-8c8a-e146b0ae0738")
}

fn brainstorm() -> baylee_core::ids::CardIndex {
    card_index("36cd2364-d113-47d1-b2c4-b088d9eb88dd")
}

/// Seat 1 casts a noncreature spell into seat 0's Esper Sentinel, and this
/// is the number the Sentinel asks them for.
///
/// Four Islands is enough for Brainstorm and the largest tax either half of
/// this pair asks, so the tax is answered out of the floating pool. A seat
/// that could not pay is asked too since CR 605.3a, and would be handed a
/// payment window; that is a different test.
#[track_caller]
fn the_tax_the_sentinel_asks_for(seed: u64, equip: bool) -> u16 {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(seed, forest())
        .battlefield(
            0,
            &[
                esper_sentinel(),
                sword_of_hearth_and_home(),
                plains(),
                plains(),
            ],
        )
        .battlefield(1, &[island(), island(), island(), island()])
        .hand(1, &[brainstorm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    if equip {
        let sword = on_battlefield(&engine, p0, sword_of_hearth_and_home()).expect("the sword");
        let sentinel = on_battlefield(&engine, p0, esper_sentinel()).expect("the sentinel");
        activate(&mut engine, p0, sword);
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![sentinel],
                },
            )
            .unwrap();
        pass_until(&mut engine, |e| {
            e.state()
                .object(sword)
                .is_some_and(|o| o.attached_to == Some(sentinel))
        });
    }
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, brainstorm());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayTax { mana },
        ..
    } = engine.pending()
    else {
        panic!("the Sentinel asks for its tax, got {:?}", engine.pending())
    };
    assert_eq!(*player, p1, "the tax is asked of the caster, not of me");
    *mana
}

fn teferi_time_raveler() -> baylee_core::ids::CardIndex {
    card_index("ae7604bb-4818-45a3-960c-cf3d83f15964")
}

/// Whether the stack has been emptied.
fn stack_is_clear(engine: &Engine<RegistryLookup>) -> bool {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Stack)
        .is_empty()
}

fn swiftfoot_boots() -> baylee_core::ids::CardIndex {
    card_index("c8b143ad-43ec-4e0d-a440-e348daa31391")
}

fn storm_of_saruman() -> baylee_core::ids::CardIndex {
    card_index("cf5f4860-e805-46a3-9352-a2c583e33403")
}

fn reflections_of_littjara() -> baylee_core::ids::CardIndex {
    card_index("c3fdfb94-2d10-4743-864c-a59fdd57d8b7")
}

/// How many permanents `seat` controls that a player would call `card`.
///
/// By **name**, because a copy of a permanent spell becomes a token as it
/// resolves (CR 707.10) and a token carries no card at all — so counting
/// cards would answer "one Elf" at a board holding two, and the copy would
/// be invisible to exactly the tests written to see it.
/// [`cardless_permanents`] is the other half of the pair: this one says how
/// many are there, that one says how many of them are tokens.
fn permanents_of(
    engine: &Engine<RegistryLookup>,
    seat: PlayerId,
    card: baylee_core::ids::CardIndex,
) -> usize {
    let printed = baylee_cards::by_index(card).map_or("", |def| def.faces[0].name);
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            engine.state().object(**id).is_some_and(|o| {
                o.controller == seat
                    && engine.state().names.get(o.characteristics().name) == printed
            })
        })
        .count()
}

/// Answers every question the stack asks until it is empty again.
///
/// [`pass_until`] passes priority and nothing else, which is enough while a
/// spell answers all its questions before it is on the stack. A copy effect
/// asks *after* that — the copying trigger picks the spell it copies, and the
/// copy may be pointed somewhere new (CR 707.10c) — so a test about copies
/// has to answer whatever arrives, in whatever order the wizard asks.
///
/// A choice made *as a permanent enters* is handed back instead: it names
/// something only the test knows — which creature type an enchantment is
/// about — and answering it with "whatever was first" would quietly decide
/// the thing under test.
#[track_caller]
fn settle(engine: &mut Engine<RegistryLookup>) {
    for _ in 0..200 {
        match engine.pending().clone() {
            Pending::ChooseSubtype { .. } => return,
            Pending::Priority { player, .. } => {
                if stack_is_empty(engine) {
                    return;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseTargets {
                player,
                options,
                min,
                ..
            } => {
                let objects = options.into_iter().take(min as usize).collect();
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects })
                    .unwrap();
            }
            other => panic!("unexpected while settling the stack: {other:?}"),
        }
    }
    panic!("the stack never emptied");
}

fn helm_of_the_host() -> baylee_core::ids::CardIndex {
    card_index("83b43aba-bf9c-4da2-967d-9daa632e97d2")
}

/// A legendary creature with a body and nothing that fires on its own: its
/// enter trigger cannot go off from a battlefield the harness laid out, and
/// its tap ability is only ever offered.
fn loran_of_the_third_path() -> baylee_core::ids::CardIndex {
    card_index("b3d81980-76f2-44e2-b1c9-01e30c726312")
}

/// Equips the Helm to Loran and walks to the beginning of combat, where its
/// trigger is waiting.
#[track_caller]
fn a_helm_on_a_legend(seed: u64, season_for: Option<usize>) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut mine = vec![
        helm_of_the_host(),
        loran_of_the_third_path(),
        plains(),
        plains(),
        plains(),
        plains(),
        plains(),
    ];
    let mut theirs = vec![ondu_cleric()];
    match season_for {
        Some(0) => mine.push(doubling_season()),
        Some(_) => theirs.push(doubling_season()),
        None => {}
    }
    let mut engine = Duel::new(seed, forest())
        .battlefield(0, &mine)
        .battlefield(1, &theirs)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let helm = on_battlefield(&engine, p0, helm_of_the_host()).expect("the Helm");
    let legend = on_battlefield(&engine, p0, loran_of_the_third_path()).expect("the legend");
    tap_mana_except(&mut engine, p0, helm);
    let equip = offered_ability(&engine, helm).expect("equip {5} is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: helm,
                ability_index: equip,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![legend],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(helm)
            .is_some_and(|o| o.attached_to == Some(legend))
    });
    pass_until(&mut engine, |e| cardless_permanents(e, p0) > 0);
    engine
}

fn emeritus_of_woe() -> baylee_core::ids::CardIndex {
    card_index("93056597-b964-421f-be2f-e92abef1c2a4")
}

/// How many *abilities* are waiting on the stack.
///
/// A trigger is an object in the stack zone like a spell is, so counting the
/// zone answers the wrong question: the test below wants to know whether the
/// Sentinel fired a second time while two spells stand there unresolved.
fn abilities_on_the_stack(engine: &Engine<RegistryLookup>) -> usize {
    let state = engine.state();
    state
        .zones
        .list(crate::zone::ZoneLocation::Stack)
        .iter()
        .filter(|id| {
            state
                .object(**id)
                .is_some_and(|o| o.kind == crate::object::ObjectKind::AbilityOnStack)
        })
        .count()
}

/// How many *spells* `seat` has standing on the stack.
fn spells_on_the_stack(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    let state = engine.state();
    state
        .zones
        .list(crate::zone::ZoneLocation::Stack)
        .iter()
        .filter(|id| {
            state
                .object(**id)
                .is_some_and(|o| o.kind == crate::object::ObjectKind::Spell && o.controller == seat)
        })
        .count()
}

/// Advances until `want` holds, answering a resolving trigger's target
/// choice on the way — [`settle`] with a stop condition of its own, because
/// these two tests want to read the stack *while* something is still on it.
#[track_caller]
fn advance_until(
    engine: &mut Engine<RegistryLookup>,
    want: impl Fn(&Engine<RegistryLookup>) -> bool,
) {
    for _ in 0..200 {
        if want(engine) {
            return;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            // One arm on purpose: both prompts are answered by
            // `ChooseObjects` over a list of object ids, and taking `min` of
            // them is the same shrug in both cases.
            Pending::ChooseTargets {
                player,
                options,
                min,
                ..
            }
            | Pending::ChooseCards {
                player,
                options,
                min,
                ..
            } => {
                let objects = options.into_iter().take(min as usize).collect();
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects })
                    .unwrap();
            }
            other => panic!("unexpected while advancing: {other:?}"),
        }
    }
    panic!("the game never reached what the test was waiting for");
}

/// Demonic Tutor, the spell Emeritus of Woe prepares. It is in nobody's
/// deck: the ability links it by `CardIndex` out of the registry.
fn demonic_tutor() -> baylee_core::ids::CardIndex {
    card_index("82004860-e589-4e38-8d61-8c0210e4ea39")
}

/// How many cards of one printing are in `seat`'s graveyard.
fn in_graveyard(
    engine: &Engine<RegistryLookup>,
    seat: PlayerId,
    card: baylee_core::ids::CardIndex,
) -> usize {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Graveyard(seat))
        .iter()
        .filter(|id| {
            engine
                .state()
                .object(**id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
        })
        .count()
}

/// Taps `count` of `seat`'s basic lands, leaving the rest untapped.
///
/// [`tap_mana_except`] empties the board but one, which is the wrong shape
/// for a test that has to float mana twice in one game: the second half
/// would find every source already tapped and read "no mana" as the answer it
/// was looking for.
#[track_caller]
fn tap_mana_count(engine: &mut Engine<RegistryLookup>, seat: PlayerId, count: usize) {
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.mana_abilities.len() >= count,
        "the board has {} untapped sources, the test wants {count}",
        legal.mana_abilities.len()
    );
    for source in legal.mana_abilities.iter().copied().take(count) {
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
}

/// Thief of Blood over a Karn, with a Doubling Season on the seat `season`
/// names.
///
/// Karn is cast rather than set up on the board, so the counters the Thief
/// drains are ones the engine placed; its loyalty is read back before the
/// Thief takes it, and the test asserts that number rather than assuming it.
/// Ten Swamps is exactly the two casts — `{4}` and then `{4}{B}{B}`, both in
/// the same main phase off one tap.
fn a_thief_over_a_karn(seed: u64, season: usize) -> (u16, (i16, i16)) {
    let p0 = PlayerId::new(0);
    let mut mine = vec![swamp(); 10];
    let mut theirs = Vec::new();
    if season == 0 {
        mine.push(doubling_season());
    } else {
        theirs.push(doubling_season());
    }
    let mut engine = Duel::new(seed, swamp())
        .battlefield(0, &mine)
        .hand(0, &[karn_the_great_creator(), thief_of_blood()])
        .battlefield(1, &theirs)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, karn_the_great_creator());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, karn_the_great_creator()).is_some() && stack_is_empty(e)
    });
    let karn = on_battlefield(&engine, p0, karn_the_great_creator()).expect("Karn arrived");
    let loyalty = engine
        .state()
        .object(karn)
        .expect("Karn")
        .counters
        .get(baylee_cards_dsl::CounterKind::Loyalty);

    cast_from_hand(&mut engine, p0, thief_of_blood());
    // Karn *leaving* is the signal, not the Thief arriving: the drain is an
    // enters-trigger, so the creature stands on the table for a priority
    // round before anything has been taken off the board. Karn is at zero
    // loyalty and dead to a state-based action the moment it has been.
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, karn_the_great_creator()).is_none() && stack_is_empty(e)
    });
    let thief = on_battlefield(&engine, p0, thief_of_blood()).expect("the Thief arrived");
    (loyalty, pt(&engine, thief))
}

/// A Spark Double copying seat 0's Llanowar Elves, with a Doubling Season on
/// the board only when `season` says so.
///
/// Their Elf is the bystander, and it is the same card as mine on purpose:
/// "a creature or planeswalker **you control**" read off the wrong seat
/// would offer a permanent that looks identical in every other way.
fn a_spark_double_copying_an_elf(seed: u64, season: bool) -> ((i16, i16), u16) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut mine = vec![island(), island(), island(), island(), llanowar_elves()];
    if season {
        mine.push(doubling_season());
    }
    let mut engine = Duel::new(seed, island())
        .battlefield(0, &mine)
        .hand(0, &[spark_double()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my elf");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their elf");

    cast_from_hand(&mut engine, p0, spark_double());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let options = target_options(&engine);
    assert!(
        options.contains(&my_elf) && !options.contains(&their_elf),
        "\"a creature or planeswalker you control\" is the whole scope: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![my_elf],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, spark_double()).is_some() && stack_is_empty(e)
    });
    let copy = on_battlefield(&engine, p0, spark_double()).expect("the copy arrived");
    let loyalty = engine
        .state()
        .object(copy)
        .expect("the copy")
        .counters
        .get(baylee_cards_dsl::CounterKind::Loyalty);
    (pt(&engine, copy), loyalty)
}

/// The cards that reach that path, and why each is not a live defect.
///
/// A name and a reason, the way `offer_tests::INERT_TOKENS` is a token and
/// the sentence excusing it: nobody grows this list without writing down
/// what they are excusing, and both halves are checked below, so an entry
/// that stops being true fails as loudly as a card that stops being listed.
///
/// It is **empty**, and it held one card until `CopyMod::KeepOtherAbilities`
/// existed. Sakashima of a Thousand Faces prints "the legend rule doesn't
/// apply to permanents you control" and says "…except it has Sakashima's
/// other abilities"; with no word for that clause it was excused here,
/// because the static nothing un-registered supplied exactly what the
/// clause keeps. The card carries the mod now and the check below skips
/// cards that do, so the excuse went with the accident.
///
/// An entry belongs here again only for a card that prints a static, becomes
/// a copy, does **not** carry the mod, and is right anyway. Write down why.
const COPIES_KEEPING_A_PRINTED_STATIC: &[(&str, &str)] = &[];

fn elspeth_storm_slayer() -> baylee_core::ids::CardIndex {
    card_index("f78af825-023a-42e9-8374-5c52303a1417")
}

fn phyrexian_metamorph() -> baylee_core::ids::CardIndex {
    card_index("340bbe8b-e987-4c3e-ab4e-9dee63e57d4f")
}

fn solemn_simulacrum() -> baylee_core::ids::CardIndex {
    card_index("00c0543c-2a1f-4425-8283-4062d74a1637")
}
