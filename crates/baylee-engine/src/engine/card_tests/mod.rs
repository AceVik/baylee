//! Behavioral card tests on the shared [`testkit`]: the pattern for the
//! card pool going forward. Each test is deliberately small — the kit
//! carries the duel plumbing, the test carries only the card's rules
//! text as a scenario.
//!
//! # Where a test sits
//!
//! A test goes with **the card it plays**, under the same door `cards/`
//! puts that card behind — a creature's scenario in `creatures`, a land's
//! in `lands`. A scenario that reaches a *rule* rather than a card is in
//! `rules`: it still plays a printing, because the engine advances no
//! other way, but the card there is an example and not the subject.
//!
//! # What stays here, and why it has to
//!
//! Every non-test item — the card handles, `activate`, `keywords`,
//! `tap_all_mana_but`, `library_size` — stays in this file. That is not tidiness
//! but visibility: a child module reaches its parent's private items through
//! `use super::*`, and a **sibling** reaches nothing at all. A helper moved
//! into `creatures` would be invisible to `lands`, and the twenty of them
//! that the pool's tests lean on are shared across every door. So the split
//! moves `#[test]` functions and nothing else, which is also what makes it
//! reviewable: no item was rewritten, only re-filed.

mod artifacts;
mod creatures;
mod enchantments;
mod instants;
mod lands;
mod planeswalkers;
mod rules;
mod sorceries;

use super::testkit::*;

use super::*;

use crate::choice::{CastModeKind, ChoicePrompt, YesNoPrompt};
use crate::object::Status;
use crate::zone::{Zone, ZoneLocation};
use baylee_cards_dsl::{CounterKind, KeywordSet};
use baylee_core::ids::{CardIndex, Defender, ObjectId};
use baylee_core::mana::ManaColor;
use baylee_core::types::{SupertypeSet, TypeSet};

fn crib_swap() -> CardIndex {
    card_index("2987c385-011a-4032-a516-a46d1e9dc9e8")
}

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}

fn earth_king_s_lieutenant() -> CardIndex {
    card_index("9da9248d-1201-447f-b6c2-2b64af4f71c4")
}

fn ondu_cleric() -> CardIndex {
    card_index("f4232466-dd6a-49bf-be6c-95905c3ded17")
}

fn counterspell() -> CardIndex {
    card_index("cc187110-1148-4090-bbb8-e205694a39f5")
}

fn silence() -> CardIndex {
    card_index("8aed54cb-d1bb-45ad-adbe-38e55d84ff31")
}

fn drannith_magistrate() -> CardIndex {
    card_index("aadd10d0-6dd0-4bdc-8d93-ff08e29a5863")
}

fn jin_gitaxias() -> CardIndex {
    card_index("f5daadc1-98ff-480a-82bb-fe7bfaa7b60e")
}

fn swords_to_plowshares() -> CardIndex {
    card_index("b1544f21-7e98-461b-aed5-e748b0168c52")
}

fn heroic_intervention() -> CardIndex {
    card_index("24882fa2-3fe9-4c1b-aa3d-0e6488b9db27")
}

fn banishing_stroke() -> CardIndex {
    card_index("a6898364-c29e-4b97-a500-344efa3ec24a")
}

fn katara_the_fearless() -> CardIndex {
    card_index("0972d46e-423b-454e-87c7-a2d40fb6fb6d")
}

fn curse_of_the_swine() -> CardIndex {
    card_index("5669ea7c-c4fc-494c-896b-4bce9b494817")
}

fn giant_growth() -> CardIndex {
    card_index("5748ebf1-24e3-499d-ab7c-c2cebd462a24")
}

fn ephemerate() -> CardIndex {
    card_index("0fd57894-b917-41c8-a394-360d1d31b236")
}

/// A permanent's projected power, which is what a pump is visible in.
fn power_of(engine: &Engine<RegistryLookup>, object: ObjectId) -> Option<i16> {
    engine
        .state()
        .object(object)
        .and_then(|o| o.characteristics().power)
}

fn storm_of_saruman() -> CardIndex {
    card_index("cf5f4860-e805-46a3-9352-a2c583e33403")
}

fn karn_the_great_creator() -> CardIndex {
    card_index("a20dd48d-d344-4db1-b0e9-a2b71c3cc9d1")
}

fn chromatic_lantern() -> CardIndex {
    card_index("539f5396-d99a-417d-a84c-dff7930b5900")
}

fn abraded_bluffs() -> CardIndex {
    card_index("ca7d093c-0533-493f-9ad3-8af30118fbfc")
}

fn treetop_village() -> CardIndex {
    card_index("b53f216d-1592-4eee-b204-502a805fbc8c")
}

fn great_divide_guide() -> CardIndex {
    card_index("79e69a91-d580-47fb-be76-1e32c50d2fa0")
}

fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}

fn badlands() -> CardIndex {
    card_index("13ff3222-91cb-4796-a34e-899ed817694c")
}

fn lightning_greaves() -> CardIndex {
    card_index("ca204b66-8d0c-431a-8d34-282f7c2d17da")
}

fn llanowar_elves() -> CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
}

fn festering_goblin() -> CardIndex {
    card_index("66fb4764-d309-4c30-a2a4-474f9030dc87")
}

/// Lotleth Troll — a 2/1 that shields itself for `{B}` and prints no
/// hexproof, which is what makes it the creature every "it can't be
/// regenerated" test kills.
// oracle_id = "61b1d7e5-6155-4204-b110-35a890551ec8"
fn lotleth_troll() -> CardIndex {
    card_index("61b1d7e5-6155-4204-b110-35a890551ec8")
}

/// Walks the game until a target question is asked and hands back the menu
/// it published, **without** answering it.
///
/// [`aim_at`] is the sibling that answers. This one exists for the tests
/// whose subject is the menu itself: a printed filter is a claim about which
/// cards are on it, and what proves the claim is the cards that are there
/// beside the one that is not.
#[track_caller]
fn pass_until_targets(engine: &mut Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on nothing else")
    };
    assert_eq!(player, seat, "the seat that cast it names the target");
    options
}

/// Puts permanents into their owner's graveyard through the destruction
/// door, without handing priority over.
///
/// The companion to [`kill`], and the difference is what the test is about.
/// `kill` is about the *dying* and lets whatever triggered resolve; this is
/// about the graveyard being full, so that a reanimation spell has something
/// to point at. `sba::destroy` moves the card on the spot, so nothing has to
/// be passed for it — and passing would spend the main phase the spell still
/// has to be cast in.
///
/// Every caller seats creatures that print no dies trigger, which is what
/// makes the shortcut honest rather than merely convenient.
#[track_caller]
fn bury(engine: &mut Engine<RegistryLookup>, ids: &[ObjectId]) {
    let state = engine
        .dev_state_mut(PlayerId::new(0))
        .expect("the harness may set boards up");
    for id in ids {
        crate::sba::destroy(state, *id);
    }
}

/// Destroys `id` the way a state-based action does, then lets the game judge
/// the board and resolve whatever died triggered.
///
/// The harness door and not a removal spell, because what these tests are
/// reading is the *dying*: a spell would put its own colours, its own cost
/// and its own target legality between the board and the sentence, and every
/// card that dies here dies the same way whatever killed it (CR 704.5g).
/// The same helper with the pass taken out is `undying_tests::kill`, where
/// the rule itself is read.
#[track_caller]
fn kill(engine: &mut Engine<RegistryLookup>, id: ObjectId) {
    let state = engine
        .dev_state_mut(PlayerId::new(0))
        .expect("the harness may set boards up");
    crate::sba::destroy(state, id);
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    engine
        .apply(player, PlayerAction::PassPriority)
        .expect("passing priority is always legal");
    pass_until(engine, stack_is_empty);
}

/// Buy one regeneration shield off `source`'s own ability, and prove it is
/// standing before handing the board back.
///
/// Bought and never written in with `dev_state_mut`: a test about a spell
/// that kills *through* a shield says nothing if the shield it ignored was
/// put there by the harness rather than by a card, because then nothing in
/// the test has read the rule at all.
#[track_caller]
fn raise_a_shield(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    source: ObjectId,
    ability_index: u32,
) {
    engine
        .apply(
            seat,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the regeneration is offered and affordable");
    pass_until(engine, |e| at_rest(e, seat));
    assert_eq!(
        engine
            .state()
            .object(source)
            .expect("the permanent that shielded itself is still there")
            .regeneration_shields,
        1,
        "one shield, standing over whatever comes next"
    );
}

fn wild_elephant() -> CardIndex {
    card_index("3b2ce431-7101-4256-827d-a14de9f867fd")
}

fn fangren_hunter() -> CardIndex {
    card_index("c5dc5546-e9e5-4b5b-b812-5716d4bdee0e")
}

fn wild_colos() -> CardIndex {
    card_index("cb6b8ce3-9f9d-418c-94b0-c4469a254938")
}

fn rib_cage_spider() -> CardIndex {
    card_index("906cba93-3dac-4720-a482-987cf1b4e786")
}

/// Answer the target question an ability has just asked, and hand back the
/// menu it offered.
///
/// Returned rather than merely answered, because on a card that names a
/// subtype the menu *is* the card: a target named out of a list nobody read
/// proves only that the engine accepted it, and a filter reaching one
/// creature too far looks exactly the same from the outside.
#[track_caller]
fn aim_at(engine: &mut Engine<RegistryLookup>, seat: PlayerId, target: ObjectId) -> Vec<ObjectId> {
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert_eq!(player, seat, "the activating seat names the target");
    assert_eq!((min, max), (1, 1), "one target, and the ability asks once");
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .expect("the named target was on the menu");
    options
}

fn aurochs() -> CardIndex {
    card_index("3961ef7c-4eb4-482e-9cda-d49d6a29c5a9")
}

fn rootbreaker_wurm() -> CardIndex {
    card_index("d3edbb47-6892-4853-badc-cc01499d4e55")
}

fn muscle_burst() -> CardIndex {
    card_index("97487ea5-2bbd-4ef6-a870-7e9f2db5e5e0")
}

fn time_sieve() -> CardIndex {
    card_index("3da5977a-36d4-4f32-ab9b-8b93809d818d")
}

fn the_meathook_massacre() -> CardIndex {
    card_index("127de52b-df75-4342-95a0-20d84c5bf916")
}

fn underworld_breach() -> CardIndex {
    card_index("27e0948b-9916-473b-8d8c-a51bdfbc7457")
}

fn profane_procession() -> CardIndex {
    card_index("a656ad7f-133f-4d93-919a-43bcf1f815f3")
}

fn retreat_to_kazandu() -> CardIndex {
    card_index("3f8e5ff1-af89-427e-924c-19a44f9a3788")
}

fn temur_ascendancy() -> CardIndex {
    card_index("e68dc47c-692f-4420-9799-eee104017273")
}

fn virtue_of_knowledge() -> CardIndex {
    card_index("f0bbcabf-29e7-4c7e-893f-86b64d3620a9")
}

fn erode() -> CardIndex {
    card_index("2e467fab-e808-44d3-99bf-e3621baeb7cb")
}

fn malakir_rebirth() -> CardIndex {
    card_index("a731e87b-8d99-4b64-8ee3-8e540d652366")
}

fn revitalizing_repast() -> CardIndex {
    card_index("8dd6d060-d023-48a6-85cb-7a5521b6257b")
}

fn spikefield_hazard() -> CardIndex {
    card_index("81036c9f-fe0a-45a7-bcd5-0d344f31055a")
}

fn vastwood_fortification() -> CardIndex {
    card_index("ce148a0c-6c63-49d5-a156-99efae4e367a")
}

fn hero_s_downfall() -> CardIndex {
    card_index("03df6a57-37c9-46d3-83b3-4a6240100714")
}

/// Casts the **front** face of a modal double-faced card off floating mana.
///
/// [`cast_from_hand`] is enough for a card with one way out of the hand; a
/// card whose other face is a land is asked which one is being played, and a
/// test that meant the spell has to say so rather than take whatever the
/// first option happens to be.
#[track_caller]
fn cast_front_face(engine: &mut Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) {
    cast_with_floating(engine, seat, card);
    if let Pending::ChooseCastMode { options, .. } = engine.pending().clone() {
        let slot = options
            .iter()
            .position(|o| matches!(o.kind, CastModeKind::Face(0)))
            .expect("the front face is one of the ways to play this card");
        engine
            .apply(seat, PlayerAction::ChooseMode(slot))
            .expect("the front face is a legal choice");
    }
}

fn sphinx_of_the_final_word() -> CardIndex {
    card_index("d4246e4d-390d-4925-a5a8-89cd096a237c")
}

fn tyrranax_rex() -> CardIndex {
    card_index("6e42da0c-151e-468d-91cb-5a5b117a9298")
}

fn maelstrom_wanderer() -> CardIndex {
    card_index("ad9b7fbc-61c8-43ee-a65c-99206fd1e4df")
}

fn rancor() -> CardIndex {
    card_index("9d2d6479-531c-4ce1-b52b-00e36fa63b64")
}

fn fastbond() -> CardIndex {
    card_index("e27193b7-1a47-4555-865d-b1fd4c6d597f")
}

fn mirri_s_guile() -> CardIndex {
    card_index("7f89c0ee-b914-406c-8a3f-98424a52ae14")
}

fn arguel_s_blood_fast() -> CardIndex {
    card_index("be2a4bc4-8af6-48c5-9421-32d26272e71a")
}

fn steely_resolve() -> CardIndex {
    card_index("48c127f0-2857-4c36-97bc-1291b6fe4a82")
}

fn sterling_grove() -> CardIndex {
    card_index("2c275a85-5a15-46cd-a6e7-add63f9b853d")
}

fn sylvan_library() -> CardIndex {
    card_index("92eed395-62ca-4293-882b-8565c40daab5")
}

fn oboro_envoy() -> CardIndex {
    card_index("be70c6e8-6f9f-49fb-ab40-c6ce0ec2077c")
}

fn thrun_the_last_troll() -> CardIndex {
    card_index("1149e5ac-554a-41b1-84ae-bac42579c1aa")
}

fn world_shaper() -> CardIndex {
    card_index("3c075bb6-1831-4521-bd8d-4ed2825ae796")
}

fn ashaya_soul_of_the_wild() -> CardIndex {
    card_index("162572f2-1757-42e9-bd97-e6bd9a762c0e")
}

fn thrun_breaker_of_silence() -> CardIndex {
    card_index("789b7af5-ac15-40b6-b5b7-f3fcdcfb52e1")
}

fn aesi_tyrant_of_gyre_strait() -> CardIndex {
    card_index("6511f317-bd38-46d0-b800-7125a3f420da")
}

fn disciple_of_freyalise() -> CardIndex {
    card_index("2699005b-a471-429f-a9d8-fbf2077ee2fd")
}

fn lumra_bellow_of_the_woods() -> CardIndex {
    card_index("97a84e9d-bfc4-4ca2-b1e8-908dba56ccdb")
}

fn muldrotha_the_gravetide() -> CardIndex {
    card_index("e4625704-1d52-44e4-804f-2f45644d76ac")
}

fn primeval_titan() -> CardIndex {
    card_index("ae83ef2c-960f-4c5b-97cc-52465c687c18")
}

fn drowner_of_truth() -> CardIndex {
    card_index("db19a27a-ee22-4931-ae3c-0ce21f456ea6")
}

fn irradiate() -> CardIndex {
    card_index("84d45389-a085-44bc-a3fb-1a5f7cc6cbe0")
}

fn feeding_frenzy() -> CardIndex {
    card_index("d14fa263-a6ae-4ab4-b391-2f1ff356fa54")
}

fn wirewood_pride() -> CardIndex {
    card_index("ff19f10c-777c-4688-b1ab-99e53afaf629")
}

fn thopter_foundry() -> CardIndex {
    card_index("88bef744-550e-4f33-b1ff-a8ee990ec754")
}

fn walking_ballista() -> CardIndex {
    card_index("4b515bb0-f275-4400-8032-3173b799ab40")
}

fn swan_song() -> CardIndex {
    card_index("8ddfc283-c9b4-41a5-af88-cf0068e986cc")
}

fn gamble() -> CardIndex {
    card_index("a54f0869-94c8-42af-9080-166efb9486a4")
}

fn teferis_protection() -> CardIndex {
    card_index("0d4ecdb1-ec90-497f-a7a4-1c68092b8757")
}

fn young_wolf() -> CardIndex {
    card_index("8b492764-10b6-4506-be11-22daa9220a91")
}

fn theorist_s_proxy() -> CardIndex {
    card_index("0089acfe-da66-4dd7-b1e5-4d7407f58257")
}

fn thorin_oakenshield() -> CardIndex {
    card_index("bdd41af0-bbd1-4ecd-a699-99f006f5e5ce")
}

fn fellwar_stone() -> CardIndex {
    card_index("95560508-7ac9-4be9-8a3f-3c7d5b52807b")
}

fn an_offer_you_cant_refuse() -> CardIndex {
    card_index("234a734b-ba28-4f1b-9d01-3c3e7d516590")
}

fn dark_ritual() -> CardIndex {
    card_index("53f7c868-b03e-4fc2-8dcf-a75bbfa3272b")
}

fn past_in_flames() -> CardIndex {
    card_index("37a18736-5fe2-4897-809b-013497bdd890")
}

/// Activates printed ability `index` of `card`.
#[track_caller]
fn activate(engine: &mut Engine<RegistryLookup>, seat: PlayerId, card: CardIndex, index: u32) {
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, ai)| {
            *ai == index
                && engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
        })
        .expect("the ability is offered");
    engine
        .apply(
            seat,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the ability activates");
}

/// The keywords a battlefield object has *after* the layer system has run,
/// which is the only reading that can see a granted one.
fn keywords(engine: &Engine<RegistryLookup>, object: ObjectId) -> baylee_cards_dsl::KeywordSet {
    engine
        .state()
        .object(object)
        .expect("object exists")
        .characteristics()
        .keywords
}

fn rogue_s_passage() -> CardIndex {
    card_index("f29dc596-2121-4421-8463-15f6c2e8b9b3")
}

fn mox_opal() -> CardIndex {
    card_index("de2440de-e948-4811-903c-0bbe376ff64d")
}

fn liquimetal_coating() -> CardIndex {
    card_index("f4bdc551-c2eb-4a34-a3e3-b4a017c925af")
}

fn sunken_hollow() -> CardIndex {
    card_index("cd2c90ac-2b04-461c-92f3-939871b6b6a3")
}

/// `Land — Plains Island`, and **nonbasic**: the bystander that separates
/// "an Island" from "a basic land".
fn irrigated_farmland() -> CardIndex {
    card_index("406eabe2-df62-49e2-bb39-c0227509d875")
}

/// Whether the land `seat` just played came in tapped.
#[track_caller]
fn entered_tapped(engine: &Engine<RegistryLookup>, land: ObjectId) -> bool {
    engine
        .state()
        .object(land)
        .expect("the land is on the battlefield")
        .status
        .contains(Status::TAPPED)
}

/// Answer a reveal land's entry question by showing `card`.
///
/// [`EnterModifier::TappedUnlessReveal`] publishes a `Pending::ChooseCards`
/// with `min: 0`, so the land is standing on the battlefield *unfinished*
/// while this is unanswered: it is neither tapped nor untapped until the
/// question is settled, and every assertion about it before that reads a
/// half-built permanent.
fn reveal_on_entry(engine: &mut Engine<RegistryLookup>, seat: PlayerId, card: ObjectId) {
    engine
        .apply(
            seat,
            PlayerAction::ChooseObjects {
                objects: vec![card],
            },
        )
        .expect("a card the entry question put on the menu is a legal answer");
}

/// Plays `card` out of `seat`'s hand and answers with the object it became.
#[track_caller]
fn play_land(engine: &mut Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> ObjectId {
    let land = in_hand(engine, seat, card).expect("the land is in hand");
    engine
        .apply(seat, PlayerAction::PlayLand { card: land })
        .unwrap();
    land
}

fn deserted_beach() -> CardIndex {
    card_index("f0ec8681-da50-466b-8cdd-1dc710deccd9")
}

/// The lands whose own enters-tapped-unless filter matches the land printing
/// it, and therefore the exact set that
/// [`a_slow_land_counts_the_other_lands_and_never_itself`] speaks for.
///
/// Both bounds over `Filter::YOUR_LAND` are in it, and they are what the
/// list is for: a slow land wants two other lands and a fast land wants at
/// most two, so one count that included the entering land would turn one
/// cycle on a land early and the other off a land early. Twenty-five cards
/// ride on the skip in `Engine::controls_count`, and Cave of the Frost
/// Dragon rides on it printing the bound as its complement.
const LANDS_THAT_WOULD_COUNT_THEMSELVES: &[&str] = &[
    "Blackcleave Cliffs",
    "Blooming Marsh",
    "Botanical Sanctum",
    "Cave of the Frost Dragon",
    "Concealed Courtyard",
    "Copperline Gorge",
    "Darkslick Shores",
    "Deathcap Glade",
    "Den of the Bugbear",
    "Deserted Beach",
    "Dreamroot Cascade",
    "Hall of Storm Giants",
    "Haunted Ridge",
    "Hive of the Eye Tyrant",
    "Inspiring Vantage",
    "Lair of the Hydra",
    "Mystic Sanctuary",
    "Overgrown Farmland",
    "Razorverge Thicket",
    "Rockfall Vale",
    "Seachrome Coast",
    "Shattered Sanctum",
    "Shipwreck Marsh",
    "Spirebluff Canal",
    "Stormcarved Coast",
    "Sundown Pass",
    "Thran Portal",
    "Witch's Cottage",
];

fn skyclave_apparition() -> CardIndex {
    card_index("d90af00a-d322-4265-9954-7b1e80702e18")
}

/// Casts the Apparition on p0's first main phase and leaves it on the
/// stack, with `their_board` standing across the table.
fn a_skyclave_over(their_board: &[CardIndex]) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(201, forest())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[skyclave_apparition()])
        .battlefield(1, their_board)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("a main phase hands priority back");
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let card = in_hand(&engine, p0, skyclave_apparition()).expect("the Apparition is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("three Plains pay {1}{W}{W}");
    engine
}

fn eerie_interlude() -> CardIndex {
    card_index("0634091a-a74c-4cea-b6d1-7324a725554a")
}

fn nephalia_drownyard() -> CardIndex {
    card_index("6429b4ed-1845-4643-9a3d-85f7c12f2bba")
}

fn blighted_gorge() -> CardIndex {
    card_index("c2cb0afd-781f-4cfa-b680-ed1edfa81868")
}

fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

fn library_size(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine.state().zones.list(ZoneLocation::Library(seat)).len()
}

fn wizard_class() -> CardIndex {
    card_index("36f68aa3-9955-46f1-bc87-497f16ef5222")
}

fn bleachbone_verge() -> CardIndex {
    card_index("2b8144a0-08d2-4c28-9fd7-5d90f90105e4")
}

/// Taps every mana source `seat` has that taps for its mana, except the
/// ones printed `skip`.
///
/// [`tap_mana_except`] keeps one object; this keeps a whole printing, which
/// is how a test says "leave the Plains for the instant I am holding".
fn tap_all_mana_but(engine: &mut Engine<RegistryLookup>, seat: PlayerId, skip: Option<CardIndex>) {
    let printed: Vec<(ObjectId, Option<CardIndex>)> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .map(|id| {
            (
                *id,
                engine
                    .state()
                    .object(*id)
                    .and_then(|o| o.card)
                    .map(|c| c.index),
            )
        })
        .collect();
    tap_mana_where(engine, seat, |id| {
        skip.is_none()
            || printed
                .iter()
                .find(|(other, _)| *other == id)
                .is_none_or(|(_, index)| *index != skip)
    });
}

fn baleful_strix() -> CardIndex {
    card_index("37688720-03de-4eca-a82d-a0afe8d58adc")
}

fn tishanas_tidebinder() -> CardIndex {
    card_index("2993dc7d-723d-4a9b-94bd-4bb02a9f7243")
}

/// A Baleful Strix under a Tishana's Tidebinder that countered its
/// enters-trigger. Answers `(engine, p0, p1, strix)` with the counter
/// resolved and the stack empty.
///
/// p0 keeps a Plains and Swords to Plowshares in reserve, for the half of
/// the sentence that asks what happens when the Tidebinder leaves.
fn a_strix_the_tidebinder_answered() -> (Engine<RegistryLookup>, PlayerId, PlayerId, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(23, island())
        .battlefield(0, &[island(), swamp(), plains()])
        .hand(0, &[baleful_strix(), swords_to_plowshares()])
        .battlefield(1, &[island(), island(), island()])
        .hand(1, &[tishanas_tidebinder()])
        .start();
    keep_mulligans(&mut engine);

    reach_main_phase(&mut engine, p0);
    let strix_card = in_hand(&engine, p0, baleful_strix()).expect("the strix is in hand");
    tap_all_mana_but(&mut engine, p0, Some(plains()));
    engine
        .apply(p0, PlayerAction::CastSpell { card: strix_card })
        .unwrap();

    // Let the Strix resolve; its enters-trigger is what the Tidebinder is
    // here for, so stop as soon as that is on the stack with p1 to answer.
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, baleful_strix()).is_some()
            && !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    let strix = on_battlefield(&engine, p0, baleful_strix()).expect("the strix landed");
    let trigger = engine.state().zones.list(ZoneLocation::Stack)[0];

    tap_all_mana_but(&mut engine, p1, None);
    let tide_card = in_hand(&engine, p1, tishanas_tidebinder()).expect("the tidebinder is in hand");
    engine
        .apply(p1, PlayerAction::CastSpell { card: tide_card })
        .unwrap();

    // The Tidebinder resolves and its own enters-trigger asks for a target.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("pass_until only stops on a target choice")
    };
    assert!(
        options.contains(&trigger),
        "the strix's enters-trigger was not offered as a target: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![trigger],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    (engine, p0, p1, strix)
}

/// The keywords of `object`, as the layers project them.
fn keywords_of(engine: &Engine<RegistryLookup>, object: ObjectId) -> baylee_cards_dsl::KeywordSet {
    engine
        .state()
        .object(object)
        .expect("the object is still there")
        .characteristics()
        .keywords
}

fn path_to_exile() -> CardIndex {
    card_index("d683d985-9888-4d21-8b5f-69e69ce4a03b")
}

/// Every land `seat` controls, in battlefield order.
fn lands_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine.state().object(*id).is_some_and(|o| {
                o.controller == seat && o.characteristics().types.contains(TypeSet::LAND)
            })
        })
        .collect()
}

fn bojuka_bog() -> CardIndex {
    card_index("04b7362d-0490-4cb0-b5d7-2a7732f659ce")
}

fn aang_and_katara() -> CardIndex {
    card_index("481c3e14-b670-4fab-aa9f-6ce5b514096d")
}

fn wartime_protestors() -> CardIndex {
    card_index("6557813b-4ee7-4881-a37c-10c8ea097360")
}

fn aminatou() -> CardIndex {
    card_index("3a30089d-cd2d-49be-9b06-7a2454117692")
}

/// The tokens `seat` controls, in arrival order.
fn tokens_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_none() && o.controller == seat)
        })
        .collect()
}

fn aether_channeler() -> CardIndex {
    card_index("fb220f46-f8b8-4804-baa4-e7d50b4871f7")
}

/// Casts Aether Channeler off three Islands and hands the engine back
/// standing on its modal trigger's question.
///
/// Four tests share it because the four things worth proving about a modal
/// trigger are one question, two answers and a mode that is not offered —
/// and until the collection arm existed, *none of them was reachable*.
/// `AbilityDef::ModalTriggered` was skipped by both loops in `trigger.rs`,
/// so the ability never became a `PendingTrigger`, never reached the stack
/// and was never asked about: the card resolved, nothing happened, and no
/// error was reported (entry 34).
#[track_caller]
fn a_modal_trigger_asks(seed: u64, opponent_board: &[CardIndex]) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(seed, island())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, opponent_board)
        .hand(0, &[aether_channeler()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, aether_channeler());
    // Priority passes until the spell resolves and its ETB trigger asks.
    for _ in 0..20 {
        if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
            return engine;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!(
                "a modal trigger asked nothing and the game moved on — got {:?}. \
                 That is entry 34: the ability is never collected, so the card \
                 resolves and does nothing at all.",
                engine.pending()
            )
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    panic!("the modal trigger never asked for its mode")
}

fn ertai_resurrected() -> CardIndex {
    card_index("3d038f7c-95fa-4b71-8f74-b9b4dd45cde0")
}

fn panharmonicon() -> CardIndex {
    card_index("76678885-3674-443d-b9a2-2a460cf6aac0")
}

fn umara_raptor() -> CardIndex {
    card_index("a58ee84f-1d9c-4924-b7b1-14a9b2ba3b98")
}

fn solitude() -> CardIndex {
    card_index("dcb9c2a7-ae54-4ddc-a567-640bf4bf4366")
}

/// Every battlefield permanent a seat controls that was printed from `card`.
///
/// [`on_battlefield`] answers the first; this answers all of them, which is
/// what taps *some* of a seat's lands and leaves the rest untapped.
fn all_on_battlefield(
    engine: &Engine<RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.controller == seat && o.card.is_some_and(|c| c.index == card))
        })
        .collect()
}

/// Attacks `defender` with `attacker`, and returns the blocks the engine
/// offers the defending seat.
///
/// A test about blocking cannot read the board for its answer: this engine
/// publishes the pairings and `apply` validates against that same
/// enumeration, so what a creature may block is `Pending::ChooseBlockers`
/// and nothing else. The attack has to be declared first, because the offer
/// does not exist until there is something to block.
#[track_caller]
fn attack_and_collect_blocks(
    engine: &mut Engine<RegistryLookup>,
    attacker: ObjectId,
    defender: PlayerId,
) -> Vec<crate::choice::BlockOption> {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert!(
        attackers.contains(&attacker),
        "the creature this test attacks with is on the offer: {attackers:?}"
    );
    engine
        .apply(
            player,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(defender))],
            },
        )
        .expect("the attacker came out of the list that offered it");
    for _ in 0..40 {
        match engine.pending().clone() {
            Pending::ChooseBlockers { blockers, .. } => return blockers,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected on the way to the blockers: {other:?}"),
        }
    }
    panic!("never reached the declare-blockers question")
}

/// A 1/1 Umara Raptor that put its own rally counter on itself, so the
/// creature standing on the battlefield is a 2/2 and the card it was
/// printed from is not.
///
/// Both tests below need exactly that: a target whose projected power and
/// whose printed power disagree, so the life gained says which of the two
/// the effect read. Only the Islands are tapped for the Raptor — a pool of
/// eight mana pays `{2}` with whatever it likes, and it spent the white the
/// second spell needs.
fn a_two_two_raptor(seed: u64, spell: CardIndex, extra: &[CardIndex]) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut board = vec![
        island(),
        island(),
        island(),
        plains(),
        plains(),
        plains(),
        plains(),
        plains(),
    ];
    board.extend_from_slice(extra);
    let mut engine = Duel::new(seed, island())
        .battlefield(0, &board)
        .hand(0, &[umara_raptor(), spell])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    for source in all_on_battlefield(&engine, p0, island()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let raptor = in_hand(&engine, p0, umara_raptor()).expect("the Raptor is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: raptor })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let bird = on_battlefield(&engine, p0, umara_raptor()).expect("the Raptor resolved");
    assert_eq!(
        engine
            .state()
            .object(bird)
            .and_then(|o| o.characteristics().power),
        Some(2),
        "the rally trigger put a +1/+1 counter on it",
    );
    for source in all_on_battlefield(&engine, p0, plains()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    engine
}

fn inspirit_flagship_vessel() -> CardIndex {
    card_index("554df866-3dbb-4811-8573-6033481591aa")
}

fn sheoldred_the_apocalypse() -> CardIndex {
    card_index("34f34409-326d-4994-a0ea-1a69aa278f03")
}

fn toxic_deluge() -> CardIndex {
    card_index("afaef788-34d1-460b-b884-9d7ae6ddeb18")
}

fn darksteel_forge() -> CardIndex {
    card_index("9b3bec05-441f-4fdf-8b51-69fa8613fcd4")
}

fn primaris_eliminator() -> CardIndex {
    card_index("7d679591-f8ea-4c4c-ab98-7b9e3438cf57")
}

fn mystical_tutor() -> CardIndex {
    card_index("fb81f95c-70f8-4eb7-8d15-15d0ae23ec03")
}

fn halimar_excavator() -> CardIndex {
    card_index("fd3e37c9-93bf-4f3e-a279-22afbffd8d43")
}

fn hagra_diabolist() -> CardIndex {
    card_index("5e2c1e0e-0a10-416a-9b50-96ee0cbbc24e")
}

fn vendilion_clique() -> CardIndex {
    card_index("244d4807-0802-41bc-9460-55ac38a28a72")
}

fn loran_of_the_third_path() -> CardIndex {
    card_index("b3d81980-76f2-44e2-b1c9-01e30c726312")
}

/// A Hagra Diabolist cast and its rally trigger waiting on a target.
fn hagra_on_the_table() -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(19, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[hagra_diabolist()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    cast_from_hand(&mut engine, p0, hagra_diabolist());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
}

fn luminarch_ascension() -> CardIndex {
    card_index("90076bf5-aa9a-4a6e-9035-9aa97fd5561e")
}

/// An Ondu Cleric cast, with its rally trigger asking whether to take the
/// life it offers.
///
/// Two tests over one builder rather than two arms of one, for the reason
/// the Hagra pair has: the second answer wants the same open board the
/// first one spent.
fn a_cleric_asking() -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    cast_from_hand(&mut engine, p0, ondu_cleric());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine
}

fn jace_the_mind_sculptor() -> CardIndex {
    card_index("7f77a84e-5a4b-4834-aefa-3cecc175ae8e")
}

fn venser_the_sojourner() -> CardIndex {
    card_index("a8bf8ff8-d924-4fd2-b5ed-05b38f55325a")
}

/// The types an object has after the layer system has run — the only
/// reading that can see a type a continuous effect added.
fn types(engine: &Engine<RegistryLookup>, object: ObjectId) -> baylee_core::types::TypeSet {
    engine
        .state()
        .object(object)
        .expect("object exists")
        .characteristics()
        .types
}

fn mycosynth_lattice() -> CardIndex {
    card_index("ae1f2ab5-c6a5-4d49-a746-3cb4668bf805")
}

fn brainstorm() -> CardIndex {
    card_index("36cd2364-d113-47d1-b2c4-b088d9eb88dd")
}

fn enlightened_tutor() -> CardIndex {
    card_index("c5229c17-b7be-4b05-b683-f2277edc4849")
}

fn arid_mesa() -> CardIndex {
    card_index("c5acf2a5-40f4-433d-a74d-1cb56c521464")
}

fn gaea_s_cradle() -> CardIndex {
    card_index("7c427c3d-ecd8-45ef-bebd-8f10f4a311db")
}

fn prairie_stream() -> CardIndex {
    card_index("5330e24a-8568-446e-840a-594cd08bd1bc")
}

fn orcish_bowmasters() -> CardIndex {
    card_index("ea5103f5-27e0-4eb1-902c-7f34652d6bf3")
}

fn mikokoro() -> CardIndex {
    card_index("a4580a1d-141e-449b-9018-e0258130634b")
}

/// Answers whatever stands between here and the next quiet priority, and
/// says whether a target was ever asked for.
///
/// Written for a *trigger* that targets, which nothing in the pool had until
/// Orcish Bowmasters: a loop that merely tolerates `ChooseTargets` passes
/// whether the question is asked or not, which is how the card sat in the
/// pool marked `Implemented` and pointed at nobody.
fn settle_aiming_at(engine: &mut Engine<RegistryLookup>, face: PlayerId) -> bool {
    let mut asked = false;
    for _ in 0..40 {
        match engine.pending().clone() {
            Pending::ChooseTargets { player, .. } => {
                asked = true;
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![face],
                        },
                    )
                    .expect("a face is a legal target for `any target`");
            }
            Pending::Priority { .. } if stack_is_empty(engine) => return asked,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("settle_aiming_at met an unexpected question: {other:?}"),
        }
    }
    panic!("settle_aiming_at did not reach quiet priority within forty actions")
}

#[test]
fn issue_173_settling_at_quiet_priority_does_not_pass_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(173, forest()).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let before = engine.snapshot_hash();
    assert!(!settle_aiming_at(&mut engine, p0));
    assert_eq!(engine.snapshot_hash(), before);
}

fn myr_retriever() -> CardIndex {
    card_index("d07d3be3-f69d-4484-8467-cffd43871788")
}

fn vindicate() -> CardIndex {
    card_index("63c1ac21-e3d8-40c2-8c09-3f31c52992ef")
}

fn ashnods_altar() -> CardIndex {
    card_index("4d18bcba-a346-445e-a182-6cc30b7e066d")
}

/// How many counters of `kind` are on `id`.
///
/// The kind is a parameter and not a second helper per counter, which is
/// what these tests need it to be: a depletion land's whole point is that
/// its counters are *not* charge counters, and asking the same question
/// twice with two nouns is how that is asserted.
///
/// It lived in `lands` until Devoted Druid wanted it — counters are not a
/// land mechanic, they are a permanent one, and a helper that two card-type
/// modules need belongs where both can see it rather than being written a
/// second time with the same words.
#[track_caller]
fn counters_on(
    engine: &Engine<RegistryLookup>,
    id: ObjectId,
    kind: baylee_cards_dsl::CounterKind,
) -> u16 {
    engine
        .state()
        .object(id)
        .expect("the card is still an object")
        .counters
        .get(kind)
}

/// Whether a permanent still on the battlefield is tapped.
///
/// Panics when the object is gone, deliberately: every caller is asking
/// about a permanent it has just put on the table, so a missing object is a
/// broken assumption and not the answer `false`.
fn is_tapped(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .expect("the permanent is still on the table")
        .status
        .contains(Status::TAPPED)
}

fn basilisk_collar() -> CardIndex {
    card_index("f5f4dd28-f4ae-4d39-b9b8-6ebfd63c93fe")
}

fn omnath_locus_of_rage() -> CardIndex {
    card_index("1816eede-c5bd-49df-958f-a3af64cb2932")
}

fn reclamation_sage() -> CardIndex {
    card_index("032ec6e2-6cc3-4a97-9cc7-3233f5e11904")
}

fn their_enchantment() -> CardIndex {
    card_index("90076bf5-aa9a-4a6e-9035-9aa97fd5561e")
}

fn sylvan_caryatid() -> CardIndex {
    card_index("13d4c46b-c2d6-44cc-a252-4a991d471854")
}

fn tatyova_benthic_druid() -> CardIndex {
    card_index("0715e860-3b3b-4331-9718-207973e94fee")
}

fn crop_rotation() -> CardIndex {
    card_index("28b46183-c62f-47b1-9fee-3ba148202cab")
}

fn heroes_downfall() -> CardIndex {
    card_index("03df6a57-37c9-46d3-83b3-4a6240100714")
}

fn grim_tutor() -> CardIndex {
    card_index("e62f8d69-a559-4f13-a5c9-5fb750b4af2c")
}

fn sylvan_scrying() -> CardIndex {
    card_index("ee24bf27-484d-4e1c-998e-6a74e3d3f6c4")
}

fn bazaar_of_baghdad() -> CardIndex {
    card_index("54022a10-c9f0-458d-a0ed-228843cd9a40")
}

fn carnage_tyrant() -> CardIndex {
    card_index("8c411f4e-a091-447c-9450-d895b10b4985")
}

fn city_of_ass() -> CardIndex {
    card_index("e043a795-6936-4d7e-9a77-e0175a27c8f5")
}

fn city_of_traitors() -> CardIndex {
    card_index("f161111d-9747-47b3-bb10-3c8bded32e21")
}

fn desolate_lighthouse() -> CardIndex {
    card_index("aa6dbdf2-2379-4ff5-8a6c-70258784dc35")
}

fn elephant_graveyard() -> CardIndex {
    card_index("8ada7388-fd8b-434c-a17a-bce19cf3e615")
}

fn geier_reach_sanitarium() -> CardIndex {
    card_index("7b9fafe7-d26a-4ed5-b4c4-ce13763770b5")
}

fn great_defender() -> CardIndex {
    card_index("c84496bc-6421-4930-a118-b0f9ee7e13f6")
}

fn mana_leak() -> CardIndex {
    card_index("c61fe162-2202-4e56-9ba0-393547f9875f")
}

fn oboro_palace_in_the_clouds() -> CardIndex {
    card_index("645fb11b-d684-4bec-8532-8fa97e8f7b28")
}

fn scavenger_grounds() -> CardIndex {
    card_index("5ece7d03-9ee7-4953-a06e-9d8e41874903")
}

fn shivan_gorge() -> CardIndex {
    card_index("e90a1381-c9c1-4f57-928c-5d19dc065274")
}

fn throne_of_the_high_city() -> CardIndex {
    card_index("9684447a-5955-4bc7-8ad0-8bb8b316873b")
}

fn treasure_vault() -> CardIndex {
    card_index("3c43efd6-b1a8-452c-ae20-9a936c3340ab")
}

fn watermarket() -> CardIndex {
    card_index("84d89a3d-4b28-4e19-8298-737ec6a06238")
}

fn wheel_of_fortune() -> CardIndex {
    card_index("a8abd966-de7b-46a3-8ac7-8747ab35653a")
}

fn witch_s_clinic() -> CardIndex {
    card_index("05899372-9784-4bdb-9c28-504c71fed906")
}

fn yavimaya_hollow() -> CardIndex {
    card_index("53d6113d-acdb-4754-9641-f7991a96c7b9")
}

fn access_tunnel() -> CardIndex {
    card_index("ed9cc560-f30b-4b60-a094-ccf93ed656a7")
}

fn alchemist_s_refuge() -> CardIndex {
    card_index("357ed28b-899f-404b-94ff-6fb2ef81d87b")
}

fn auriok_bladewarden() -> CardIndex {
    card_index("2884e332-707c-4a97-adc1-e1317a513ff4")
}

fn bloom_tender() -> CardIndex {
    card_index("0c23fefe-9891-4dd8-9bb1-eebdb3274e31")
}

fn cinder_marsh() -> CardIndex {
    card_index("6f8cc374-e76c-4bfa-bf20-28dea0bfefbe")
}

fn cloudcrest_lake() -> CardIndex {
    card_index("8df14d53-472c-416e-93c6-6c0b7f9b614e")
}

fn drownyard_temple() -> CardIndex {
    card_index("c30f9be4-c274-4ad0-b5d7-7d3421aa4277")
}

fn forgotten_monument() -> CardIndex {
    card_index("71393988-ad6f-43fd-9978-c0de15ae8e87")
}

fn gavony_township() -> CardIndex {
    card_index("8a44e4e7-dfa2-427b-bbff-11c398fa60bb")
}

fn hammerheim() -> CardIndex {
    card_index("c7476beb-7923-4994-8476-bc69187ecb72")
}

fn hunger_of_the_nim() -> CardIndex {
    card_index("1af3c6ff-2884-4d8c-a01f-6f74d8ea10cc")
}

fn imperial_seal() -> CardIndex {
    card_index("16cd0b90-f70c-4efa-b252-8de8784ef9a3")
}

fn labyrinth_of_skophos() -> CardIndex {
    card_index("9ec5a487-d8ed-459a-8f58-56f6e9a2dfe8")
}

fn lantern_lit_graveyard() -> CardIndex {
    card_index("73a39a1b-2fb7-4328-8718-18569ae28e9e")
}

fn library_of_alexandria() -> CardIndex {
    card_index("2111588d-9af5-4a33-989e-b074d83f0463")
}

fn mogg_hollows() -> CardIndex {
    card_index("1745fd57-467c-45f9-a46e-b9a2af87ec87")
}

fn novijen_heart_of_progress() -> CardIndex {
    card_index("b3b5137d-0225-4dba-9231-d235ab0f137c")
}

fn pillar_of_the_paruns() -> CardIndex {
    card_index("677b8ce7-f922-4ee3-b311-f199da9b352b")
}

fn pinecrest_ridge() -> CardIndex {
    card_index("d8ef7c7b-0201-4978-ac73-fd376a19830f")
}

fn rix_maadi_dungeon_palace() -> CardIndex {
    card_index("a1bee68d-135b-4e30-8830-48a3315d13a9")
}

fn rootwater_depths() -> CardIndex {
    card_index("2d28c83a-7415-4eb0-95a6-6245f2169d17")
}

fn stensia_bloodhall() -> CardIndex {
    card_index("8220c5fa-28dc-40d0-a38a-d8eefc2795d6")
}

fn strength_of_cedars() -> CardIndex {
    card_index("61b29c75-00d0-4ddb-9e27-cfd47302830e")
}

fn sunscorched_desert() -> CardIndex {
    card_index("256b8c23-589e-429d-9e6e-433d55079eb4")
}

fn thalakos_lowlands() -> CardIndex {
    card_index("5a54d6a3-b1d0-42fe-9531-604b34d197f1")
}

fn tomb_of_the_spirit_dragon() -> CardIndex {
    card_index("22f6391e-2634-440f-af1b-9581d1bff818")
}

fn tranquil_garden() -> CardIndex {
    card_index("d9dfef08-b824-4d56-a0e9-3dcefb7e4612")
}

fn trenchpost() -> CardIndex {
    card_index("42f1ccb8-eda0-4828-ac07-82d4e950d7e1")
}

fn underdome() -> CardIndex {
    card_index("8375aaaa-edc2-4a0c-98f6-af07d61ebd0a")
}

fn urborg() -> CardIndex {
    card_index("b6114962-035e-4e7f-9009-4739bf83a05a")
}

fn urza_s_power_plant() -> CardIndex {
    card_index("e11966cd-2ee3-4df4-b099-abf42dcdf0db")
}

fn vec_townships() -> CardIndex {
    card_index("b0a4680f-9707-431c-b5d5-7d4424783602")
}

fn waterveil_cavern() -> CardIndex {
    card_index("3debfa0d-9945-4a85-a714-7c3d3d74de4e")
}

fn winding_canyons() -> CardIndex {
    card_index("622e2561-48b1-4aca-9abb-9a3c284dcceb")
}

fn barkchannel_pathway() -> CardIndex {
    card_index("59d22de5-e310-44d7-89cf-ef3529e40cef")
}

fn blighted_fen() -> CardIndex {
    card_index("b8f3da11-7c8f-4846-98a6-204bfd8d572b")
}

fn blighted_steppe() -> CardIndex {
    card_index("db16a2fb-dc42-4086-9928-52076043097f")
}

fn blightstep_pathway() -> CardIndex {
    card_index("e580a229-e800-4746-9d37-c32fcef8de28")
}

fn bonders_enclave() -> CardIndex {
    card_index("f33ce38a-34ec-4b65-a0fc-160484a02007")
}

fn branchloft_pathway() -> CardIndex {
    card_index("7c304547-a4b1-46c9-baed-16d2bfbe16eb")
}

fn command_beacon() -> CardIndex {
    card_index("7e8c2a18-e404-40ff-a9e0-ec3eeb6d576e")
}

fn cragcrown_pathway() -> CardIndex {
    card_index("727ca426-f4cc-4218-8ae5-8c427af2e816")
}

fn crypt_of_agadeem() -> CardIndex {
    card_index("4fe8af73-c84a-44bd-9739-ee5c8b027874")
}

fn darkbore_pathway() -> CardIndex {
    card_index("868e6e68-4367-4073-a864-235d5961ae56")
}

fn drannith_ruins() -> CardIndex {
    card_index("d1f10cca-8dfa-4ea5-b227-4446cd8514a8")
}

fn dread_statuary() -> CardIndex {
    card_index("a9789ce4-69cf-435c-b99a-78a21609830c")
}

fn eiganjo_castle() -> CardIndex {
    card_index("895a0e00-20a9-44f8-9215-66edcdf016b7")
}

fn emergence_zone() -> CardIndex {
    card_index("7536eb66-959d-4dca-9b75-895572ef733c")
}

fn ghost_town() -> CardIndex {
    card_index("f2c861d3-b302-4e84-b647-099551007269")
}

fn hall_of_heliod_s_generosity() -> CardIndex {
    card_index("2fc070dc-f2f7-4648-8069-31d74790a39c")
}

fn kessig_wolf_run() -> CardIndex {
    card_index("c6911265-54ef-4c16-bcf2-1ffb24b7d426")
}

fn miren_the_moaning_well() -> CardIndex {
    card_index("03fe19bb-8e22-4030-8299-2ddd2d5a7eb2")
}

fn needleverge_pathway() -> CardIndex {
    card_index("a9b8d020-4d72-4934-8942-df29ef19fc1d")
}

fn ominous_cemetery() -> CardIndex {
    card_index("d002391f-1dad-4966-ac36-56cc3ec015b2")
}

fn petrified_field() -> CardIndex {
    card_index("c4bc5bc4-e589-42c5-91fa-2ebc96448e85")
}

fn prahv_spires_of_order() -> CardIndex {
    card_index("37ff5ba6-0763-4c73-85bf-66856e67b8f3")
}

fn r_d_s_secret_lair() -> CardIndex {
    card_index("b6be7abe-cee3-418f-bf52-8b5405e3462f")
}

fn rainbow_vale() -> CardIndex {
    card_index("76695b15-d0ba-41eb-85f1-52ba5d14b8ba")
}

fn rhystic_cave() -> CardIndex {
    card_index("609fbc2c-514a-4feb-aaad-b9e6dcfd335c")
}

fn riftstone_portal() -> CardIndex {
    card_index("8d7e05ba-5406-4d5e-bb8f-a4a6f3b0eaa7")
}

fn riverglide_pathway() -> CardIndex {
    card_index("4924b3a4-a218-4783-8a4d-82361fdecc78")
}

fn sandstorm_verge() -> CardIndex {
    card_index("de417a82-8f03-4d7e-aee7-48f7d7eba61a")
}

fn smoldering_spires() -> CardIndex {
    card_index("cfa3288d-e521-4a13-bcb3-7950a94e1746")
}

fn soulstone_sanctuary() -> CardIndex {
    card_index("3c0f99b8-0222-4fac-932a-eb5d77826564")
}

fn terrain_generator() -> CardIndex {
    card_index("a949c96c-362c-45a3-bd5c-ce5ce153ee9e")
}

fn the_tabernacle_at_pendrell_vale() -> CardIndex {
    card_index("69b409b3-fa16-4c79-8b46-215a7036ed46")
}

fn thespian_s_stage() -> CardIndex {
    card_index("b01e698b-608a-4fc7-8073-b01d044743ec")
}

fn unholy_grotto() -> CardIndex {
    card_index("c28211c6-a5ee-40c3-bb6a-da3e7e73fd95")
}

fn unstable_frontier() -> CardIndex {
    card_index("495214b5-2eab-4fe4-8879-a30a57a67163")
}

fn untaidake_the_cloud_keeper() -> CardIndex {
    card_index("362f25a6-01ff-4c53-be52-c6346a9b0065")
}

fn urza_s_mine() -> CardIndex {
    card_index("33e85a8a-86df-4cdc-a9cc-8cbabe92c3c0")
}

fn urza_s_tower() -> CardIndex {
    card_index("32fbb638-ab14-4e8b-a07a-d4c44e3496f2")
}

fn vault_of_the_archangel() -> CardIndex {
    card_index("eeaac65a-3480-475a-bb28-e6375d53f487")
}

fn wintermoon_mesa() -> CardIndex {
    card_index("a4a6f95e-856c-4eb5-82ba-b2406be22b23")
}

fn aether_hub() -> CardIndex {
    card_index("61c89b11-65c9-4fda-bbcd-d84de25df801")
}

fn agna_qel_a() -> CardIndex {
    card_index("22d0a848-2126-48f0-9050-38daaf93b1d0")
}

fn ancient_amphitheater() -> CardIndex {
    card_index("7211221d-d4d8-4bbe-9d2a-b82e005bfe8a")
}

fn auntie_s_hovel() -> CardIndex {
    card_index("245469ff-72b6-4846-8a82-a1d29f4d09bb")
}

fn cradle_of_the_accursed() -> CardIndex {
    card_index("36d06c91-5080-4f97-8e4c-ca8ac390e808")
}

fn crucible_of_worlds() -> CardIndex {
    card_index("33c722cf-b4bf-431f-aefd-ee96241a7fbf")
}

fn dark_fortress() -> CardIndex {
    card_index("40760bfa-a423-487c-ba29-043b2d00c736")
}

fn desert() -> CardIndex {
    card_index("195107ad-879d-4b02-a44a-a3ba70fedf88")
}

fn dunes_of_the_dead() -> CardIndex {
    card_index("c761f71c-785c-4533-a2b7-2da3667688b8")
}

fn exploration() -> CardIndex {
    card_index("0c2841bb-038c-4fbf-8360-bc0a1522b58d")
}

fn gathering_place() -> CardIndex {
    card_index("36b58705-c5a5-4547-8d8b-a7c35e1f69ae")
}

fn gilt_leaf_palace() -> CardIndex {
    card_index("85573a3d-2993-491a-8f8d-bbdb844fa84e")
}

fn gleaming_bastion() -> CardIndex {
    card_index("7785ffd4-f169-475d-9558-ce4877b3378a")
}

fn glimmervoid() -> CardIndex {
    card_index("b92e9854-4527-4133-8615-e282a213e7e3")
}

fn gods_eye_gate_to_the_reikai() -> CardIndex {
    card_index("a66008c9-1ede-4dcf-8d35-6c0ed2390996")
}

fn gond_gate() -> CardIndex {
    card_index("4306938b-c0db-4e63-a4fb-61628e5ff41f")
}

fn grasping_dunes() -> CardIndex {
    card_index("47d16c11-3033-44f3-9a12-2daf3453cc5b")
}

fn great_hall_of_the_citadel() -> CardIndex {
    card_index("b3e28bcf-0ed0-4406-b615-68ddc55b349a")
}

fn hall_of_the_bandit_lord() -> CardIndex {
    card_index("32fe7ac4-86f5-44af-9f73-ee8f6a9ce2ba")
}

fn haunted_fengraf() -> CardIndex {
    card_index("7c6143f3-ad2c-4d7f-9041-aa59f01d8fb7")
}

fn hidden_lair() -> CardIndex {
    card_index("7069d241-4e66-40bf-afd1-551a4a5457f0")
}

fn lotus_field() -> CardIndex {
    card_index("134d5b82-7940-4b33-a922-7f9d1f403e50")
}

fn moorland_haunt() -> CardIndex {
    card_index("5324192b-6687-41e4-8e56-326b21a5dbf3")
}

fn mutavault() -> CardIndex {
    card_index("6b3cc59a-7ea5-4eb5-9bf9-5a9c07f80e2b")
}

fn oran_rief_the_vastwood() -> CardIndex {
    card_index("e88027a6-24cc-4a8b-86db-734f26149ea8")
}

fn quicksand() -> CardIndex {
    card_index("ef2bb4fa-f292-4d19-aaa4-cfbe445caf45")
}

fn ramunap_excavator() -> CardIndex {
    card_index("4f819ba4-52ef-4fdd-8e4c-5ae3b2f44db5")
}

fn ramunap_ruins() -> CardIndex {
    card_index("d0d35864-1edc-4af1-9b89-3d7e94908011")
}

fn sanctum_of_eternity() -> CardIndex {
    card_index("c7d9ff27-f1fc-42e4-a47b-d2e6d68e4035")
}

fn sapseep_forest() -> CardIndex {
    card_index("8d4dcab0-86e5-4ff8-a90f-78a062664e16")
}

fn sea_gate_wreckage() -> CardIndex {
    card_index("91f34686-cb96-49c0-b4a7-49dd1fd076e2")
}

fn secluded_glen() -> CardIndex {
    card_index("09f52275-99e6-45e0-b2db-cafe26d5fb91")
}

fn secret_base() -> CardIndex {
    card_index("1017088c-08a3-45d9-a7f7-01fb2f309717")
}

fn skycoach_waypoint() -> CardIndex {
    card_index("2ac2b815-2d72-48e6-b43a-18884a74bf95")
}

fn stalking_stones() -> CardIndex {
    card_index("f3658894-3d3d-4cd4-b0ac-c53e1d08747c")
}

fn thran_quarry() -> CardIndex {
    card_index("57b4da3f-361a-4cbe-b77f-190ec33eefd8")
}

fn training_compound() -> CardIndex {
    card_index("99c70f4e-de8a-426d-99aa-17b2f87625ba")
}

fn war_room() -> CardIndex {
    card_index("71c52bf5-2a5d-488e-8b15-7ef290e4b77d")
}

fn arcane_lighthouse() -> CardIndex {
    card_index("30ac68e6-160a-41f9-9f0f-0e0eef383150")
}

fn boseiju_who_shelters_all() -> CardIndex {
    card_index("36937483-30cb-449a-8028-75017a124922")
}

fn castle_embereth() -> CardIndex {
    card_index("91fbb25b-8521-483f-88b0-77778d25f7fd")
}

fn cathedral_of_war() -> CardIndex {
    card_index("5ff647e4-730a-498f-8f2c-5bd64d5a9780")
}

fn choked_estuary() -> CardIndex {
    card_index("d473b507-8c33-4118-bc10-b0a268776074")
}

fn corrupted_crossroads() -> CardIndex {
    card_index("6276a985-7630-476d-a94f-6c6adc88f6c4")
}

fn eldrazi_temple() -> CardIndex {
    card_index("7fab8d65-af51-47d3-8f10-2676bf6e8ba3")
}

fn forbidden_orchard() -> CardIndex {
    card_index("cfd60d1f-9832-4408-b84e-0fd3018b015b")
}

fn foreboding_ruins() -> CardIndex {
    card_index("5c87e2fa-77f1-4978-b25f-f14d227301d1")
}

fn fortified_village() -> CardIndex {
    card_index("56f1a16a-9f41-41fb-b580-c200bca27cd6")
}

fn frostboil_snarl() -> CardIndex {
    card_index("7137aae6-260d-41de-8b4e-42a8cf752697")
}

fn furycalm_snarl() -> CardIndex {
    card_index("651dea9c-2375-4e44-8e65-ba8e40f0c0ef")
}

fn game_trail() -> CardIndex {
    card_index("00de57d2-7cb6-4337-9bc6-f6711e4dfabf")
}

fn halimar_depths() -> CardIndex {
    card_index("42d121a2-5266-483a-ab16-e0a8073cd6a3")
}

fn hostile_desert() -> CardIndex {
    card_index("41459587-7509-404e-bd7d-fb8831dee789")
}

fn leechridden_swamp() -> CardIndex {
    card_index("d83c86c1-126d-49e9-9b13-9e55784c49c5")
}

fn madblind_mountain() -> CardIndex {
    card_index("0ee0b090-3f1e-49d6-bcad-91e0cf1d12ae")
}

fn memorial_to_folly() -> CardIndex {
    card_index("2bc38f14-0314-4351-8138-e2b8bf041404")
}

fn mortuary_mire() -> CardIndex {
    card_index("1b3fb20a-e090-4286-9c03-6b71c27c45be")
}

fn mouth_of_ronom() -> CardIndex {
    card_index("7c05d239-39fc-4d34-a853-e3d591f4a235")
}

fn necroblossom_snarl() -> CardIndex {
    card_index("761ee6f9-b0fa-43c9-8d1f-9591ea18e52d")
}

fn nesting_grounds() -> CardIndex {
    card_index("d27bb97d-286b-4947-8d7b-443e4df93319")
}

fn port_town() -> CardIndex {
    card_index("458d2b12-f578-4392-98d3-c3bc83f316c4")
}

fn roadside_reliquary() -> CardIndex {
    card_index("2fb13687-0518-4ba0-a5ae-dd609464b026")
}

fn secret_tunnel() -> CardIndex {
    card_index("632e2979-d88a-482e-9bb8-57b683c5310f")
}

fn sequestered_stash() -> CardIndex {
    card_index("b6fe779f-b20d-49cc-96dd-54f1ffb312e1")
}

fn shineshadow_snarl() -> CardIndex {
    card_index("c9fc13d6-bd10-47bc-b2b6-7f67a1f3371e")
}

fn shrine_of_the_forsaken_gods() -> CardIndex {
    card_index("8ea46945-d5ab-4209-b473-4769e7b8b962")
}

fn springjack_pasture() -> CardIndex {
    card_index("9eaadbbc-818b-4c21-9d4b-1bba48504d38")
}

fn starting_town() -> CardIndex {
    card_index("d04e0975-f401-41b8-a9db-9bcf9cbbce66")
}

fn tectonic_edge() -> CardIndex {
    card_index("4927150d-7ff6-4232-b20e-d2ea245ac710")
}

fn the_gold_saucer() -> CardIndex {
    card_index("93e38650-ce22-4ab9-b79d-cc7b6477c075")
}

fn the_grey_havens() -> CardIndex {
    card_index("a1a9695e-073b-4a65-b3ec-2cfddc23202a")
}

fn undiscovered_paradise() -> CardIndex {
    card_index("76c33d54-ce55-400e-bec5-79d33a5a20fb")
}

fn urza_s_workshop() -> CardIndex {
    card_index("71099427-e110-488f-ab29-7867241fc7f0")
}

fn vineglimmer_snarl() -> CardIndex {
    card_index("33f52df8-4b44-4422-8b0a-37fead9c894b")
}

fn wanderwine_hub() -> CardIndex {
    card_index("c3b46bd6-b3ef-452d-a916-995c44f1da07")
}

fn zoetic_cavern() -> CardIndex {
    card_index("3763de30-28e1-4689-a71c-07d2fea3a466")
}

fn adagia_windswept_bastion() -> CardIndex {
    card_index("70d35dbd-1d91-4a2a-a643-6870d168f4f5")
}

fn avengers_tower() -> CardIndex {
    card_index("c5fc8e7c-a87e-4586-a13c-d30e0a3aafbf")
}

fn ba_sing_se() -> CardIndex {
    card_index("de1ae205-ca5b-4d26-8194-ca85f1406e53")
}

fn balamb_garden_see_d_academy() -> CardIndex {
    card_index("8b84fec5-617c-4088-8250-2ba1f1f9479a")
}

fn bucolic_ranch() -> CardIndex {
    card_index("8f5902bf-4bc4-4d0c-84ea-a425307a4eb2")
}

fn chocobo_camp() -> CardIndex {
    card_index("ed77fdf2-59c0-4310-9b12-80d28beeaeef")
}

fn demolition_field() -> CardIndex {
    card_index("93953926-a644-49bb-9b5a-4c8f19114c7e")
}

fn diamond_city() -> CardIndex {
    card_index("88e29d50-1680-495d-be84-b92b4c9e636f")
}

fn forsaken_crossroads() -> CardIndex {
    card_index("c70598e1-30c6-4f92-a265-34a7a73bc2b8")
}

fn frostwalk_bastion() -> CardIndex {
    card_index("ae4a18ec-70a3-4d21-b9e5-b13ab4901600")
}

fn glacial_chasm() -> CardIndex {
    card_index("73e7a2ad-d11c-4867-b97d-f971809da778")
}

fn great_hall_of_the_biblioplex() -> CardIndex {
    card_index("a8c70dab-1e27-4a9c-bd2d-910d5720d02d")
}

fn hall_of_storm_giants() -> CardIndex {
    card_index("087c8c0e-a91c-4e3c-8387-9312db01f343")
}

fn havengul_laboratory() -> CardIndex {
    card_index("e71ac446-02a4-4468-8d29-f28b21617665")
}

fn hidden_cataract() -> CardIndex {
    card_index("927979d7-9b5c-4448-aef0-baf2907a89f1")
}

fn hidden_courtyard() -> CardIndex {
    card_index("e19d5071-4ea1-4883-b067-a21e553f96e0")
}

fn hidden_necropolis() -> CardIndex {
    card_index("f780ee53-62b0-4c32-b5b7-047651f48e5f")
}

fn hidden_nursery() -> CardIndex {
    card_index("1a26e2d6-6bfc-4cdc-9bd6-8b37a9be2961")
}

fn hidden_volcano() -> CardIndex {
    card_index("a1c7cd7a-0795-4135-b787-effeb981d95b")
}

fn hostile_hostel() -> CardIndex {
    card_index("1b340f71-502f-48e9-85ed-9af62f356115")
}

fn kavaron_memorial_world() -> CardIndex {
    card_index("4fa826ca-d361-4391-ad0d-989ebcfa4a91")
}

fn lupinflower_village() -> CardIndex {
    card_index("b6c7c708-5212-4100-b954-b77855b27915")
}

fn monumental_henge() -> CardIndex {
    card_index("c48df45c-3513-4d56-aed6-30c2f3a759cd")
}

fn mosswort_bridge() -> CardIndex {
    card_index("7cb9e29f-835f-4155-a2a5-4b778866c773")
}

fn shelldock_isle() -> CardIndex {
    card_index("f748b2fb-6c2a-400a-8e96-fa4e4a1dfe80")
}

fn spawning_pool() -> CardIndex {
    card_index("f3bf22cf-0a6f-4fb6-ba82-63ce290308d6")
}

fn spymaster_s_vault() -> CardIndex {
    card_index("69ddca4b-5cc0-45f3-b2e6-a047c8d601be")
}

fn susur_secundi_void_altar() -> CardIndex {
    card_index("50d6cadc-07e4-479e-90f4-e3a20f769bab")
}

fn the_world_tree() -> CardIndex {
    card_index("3437d504-bf62-4c27-b15f-f6330182ff7e")
}

fn villainous_hideout() -> CardIndex {
    card_index("cd2888aa-71f3-47ee-ba33-7bb95d5bc836")
}

fn voldaren_estate() -> CardIndex {
    card_index("fb0c0426-f1a6-4e52-9242-627786d3119a")
}

fn windbrisk_heights() -> CardIndex {
    card_index("3589bcfc-42b0-414a-adce-bc690dc631c8")
}

fn archway_of_innovation() -> CardIndex {
    card_index("bfa20bc7-4626-4a52-87f4-6e2763cb8ed5")
}

fn arena_of_glory() -> CardIndex {
    card_index("63dfe794-5f56-41ec-9883-5523b41cc3e0")
}

fn balduvian_trading_post() -> CardIndex {
    card_index("7647940e-c99c-401c-ad1d-9ec730f66b6f")
}

fn blast_zone() -> CardIndex {
    card_index("393a254f-be31-431a-9341-a51286f8cbce")
}

fn branch_of_vitu_ghazi() -> CardIndex {
    card_index("7a30316b-dcd5-4a4b-b959-eecde7ca92e7")
}

fn cactus_preserve() -> CardIndex {
    card_index("8da29533-f389-4bc2-ab9b-b469f893a362")
}

fn clive_s_hideaway() -> CardIndex {
    card_index("283f743f-6e79-49de-b7ed-08e6ffb64cc6")
}

fn country_roads() -> CardIndex {
    card_index("c5a39f76-dd1b-442c-9f52-08561ecb91ad")
}

fn dalkovan_encampment() -> CardIndex {
    card_index("33a90122-7280-4481-9b97-5879194cae40")
}

fn den_of_the_bugbear() -> CardIndex {
    card_index("f451b8f0-1ff5-4e8d-9f30-9352d83ed687")
}

fn eclipsed_realms() -> CardIndex {
    card_index("5715ed43-395c-4877-99a7-8e28e7bf9dce")
}

fn fertile_thicket() -> CardIndex {
    card_index("7ba580c9-f933-43d9-b03d-a349faa6c641")
}

fn foul_roads() -> CardIndex {
    card_index("d4e4c8a5-e97b-4295-a403-d17834f73502")
}

fn gallifrey_council_chamber() -> CardIndex {
    card_index("26e4b49e-77e7-41d9-94c5-924669a82591")
}

fn hellion_crucible() -> CardIndex {
    card_index("c238ef51-4b46-43d5-a70b-40270a96a1fd")
}

fn hive_of_the_eye_tyrant() -> CardIndex {
    card_index("d17163d4-dd43-4de6-b7cf-576448160b7f")
}

fn lazotep_quarry() -> CardIndex {
    card_index("0d2fa39a-9cac-4a1f-bb1e-b6162e6d5169")
}

fn memorial_to_unity() -> CardIndex {
    card_index("a74494ef-aa35-4830-9b4c-47bff5270efc")
}

fn minas_morgul_dark_fortress() -> CardIndex {
    card_index("867dbd5a-c3cf-41ce-980b-c9babc6f30f2")
}

fn mirrex() -> CardIndex {
    card_index("5502741a-e3b9-454e-8121-4360a6db6750")
}

fn mirrorpool() -> CardIndex {
    card_index("57b86d5c-3269-44bc-a838-3c5439d820d9")
}

fn opal_palace() -> CardIndex {
    card_index("aa6723a2-75da-49f5-a1ba-cbfa82c55301")
}

fn plaza_of_heroes() -> CardIndex {
    card_index("9c58d241-4d9f-4b46-b8ee-f4587f9acfd6")
}

fn primal_beyond() -> CardIndex {
    card_index("541744d9-449d-420a-a5a1-2fffba18450f")
}

fn reef_roads() -> CardIndex {
    card_index("32438050-5ae7-4c19-bcaf-5a07a673e0e0")
}

fn restless_cottage() -> CardIndex {
    card_index("7e16595f-bdeb-422e-b99a-bfc0ed52e9f8")
}

fn restless_fortress() -> CardIndex {
    card_index("8b3726f1-20b8-42ec-8f9b-b361515c3f05")
}

fn restless_prairie() -> CardIndex {
    card_index("c071257a-63e7-48d0-a677-0b396a09b624")
}

fn restless_ridgeline() -> CardIndex {
    card_index("4c0f4a63-586a-4dde-9621-b0dd9118b2e5")
}

fn restless_spire() -> CardIndex {
    card_index("0ca4e80e-c19c-4b74-b531-c5a4dc5a8ba9")
}

fn restless_vents() -> CardIndex {
    card_index("696e7ddb-bdc7-40ee-bc5c-59e98f4a7401")
}

fn restless_vinestalk() -> CardIndex {
    card_index("0935faa2-fb90-48db-8a92-906ba0f374c7")
}

fn rocky_roads() -> CardIndex {
    card_index("a659c29f-aaca-44c5-8426-cdafcb195f86")
}

fn shifting_woodland() -> CardIndex {
    card_index("7c2a4fe5-43e8-4e20-bef2-0278d18afc4b")
}

fn sunken_palace() -> CardIndex {
    card_index("c098c507-5154-423a-a70b-f6dfd4959cf6")
}

fn the_biblioplex() -> CardIndex {
    card_index("86ed6073-c35c-4d29-9911-5fe191dd875f")
}

fn thran_portal() -> CardIndex {
    card_index("926ce6a2-7bdd-4380-ac65-bc902ba0c284")
}

fn trenzalore_clocktower() -> CardIndex {
    card_index("69143645-97b6-4c7c-9fa2-844fb3b99822")
}

fn wild_roads() -> CardIndex {
    card_index("36fbc8ba-bb4c-4e5e-9031-78c36e376851")
}

fn ash_barrens() -> CardIndex {
    card_index("58257464-278e-45fa-8e0b-bcd9a7500bc1")
}

fn big_apple_3_a_m() -> CardIndex {
    card_index("dd01ef1f-f6be-498f-82e0-dc04833e685f")
}

fn captivating_cave() -> CardIndex {
    card_index("4c77767a-8133-43bc-b7a5-09a73259d354")
}

fn cave_of_temptation() -> CardIndex {
    card_index("75540897-53f6-433b-bd70-9851551df6ef")
}

fn crawling_barrens() -> CardIndex {
    card_index("dfe1a112-97aa-4e81-8431-81552ba2cdcf")
}

fn daily_bugle_building() -> CardIndex {
    card_index("483e0c6c-8131-486c-b482-cc3396c9786b")
}

fn eden_seat_of_the_sanctum() -> CardIndex {
    card_index("84856b92-5ce8-47f3-9a1c-78d6a3e26aca")
}

fn elvenking_s_halls() -> CardIndex {
    card_index("a91e0154-14a9-4681-8236-04db231592a4")
}

fn eye_of_ugin() -> CardIndex {
    card_index("10d13ff6-c4d0-4753-8939-a8a90f0e92bb")
}

fn fabled_passage() -> CardIndex {
    card_index("0c85b8f7-0bd0-4680-9ec5-d4b110460a54")
}

fn forsaken_city() -> CardIndex {
    card_index("6bb00a28-8b5a-4049-93b7-3db02de88aeb")
}

fn goblin_town() -> CardIndex {
    card_index("98d20908-6d68-4d31-b719-207f93c9b402")
}

fn interplanar_beacon() -> CardIndex {
    card_index("073169f2-da3a-4a93-8c01-b3fd8558d225")
}

fn karn_s_bastion() -> CardIndex {
    card_index("9fb8cd81-403a-4988-8f1c-b8eccf8abd9c")
}

fn keldon_megaliths() -> CardIndex {
    card_index("ec0ea7f7-52ce-40d1-b34c-e36dd4b26120")
}

fn krosan_verge() -> CardIndex {
    card_index("d9a10971-f32b-4978-952d-fed0a5bc9e36")
}

fn lindblum_industrial_regency() -> CardIndex {
    card_index("4cc014f3-05e0-442e-9dee-03eab1aa65a3")
}

fn llanowar_reborn() -> CardIndex {
    card_index("92acb789-0e42-465c-ac16-40fefec48805")
}

fn mariposa_military_base() -> CardIndex {
    card_index("f1e03d99-024a-430b-9342-ffd2268bd103")
}

fn minas_tirith() -> CardIndex {
    card_index("7b0d7e62-0287-454a-8702-b0bfa7b41245")
}

fn mishra_s_factory() -> CardIndex {
    card_index("5963e0ef-e0bc-4611-ad4f-813a4c0eacfb")
}

fn moonring_island() -> CardIndex {
    card_index("cf620c66-7db1-4db8-ae56-ee4bc2f77d74")
}

fn nephalia_academy() -> CardIndex {
    card_index("3b7e7a11-bf59-413d-8796-640d17c2c1c6")
}

fn pendelhaven() -> CardIndex {
    card_index("f70e72e1-9abe-485b-9fea-e8b35352f5b3")
}

fn plaza_of_harmony() -> CardIndex {
    card_index("5ff1d6d8-8cea-4a25-90d9-b575f4c99bc8")
}

fn power_depot() -> CardIndex {
    card_index("64687880-03f9-4f38-985b-1027c797e33f")
}

fn river_of_tears() -> CardIndex {
    card_index("8a83d284-75a0-4901-b7d9-c4b7586ee327")
}

fn valakut_the_molten_pinnacle() -> CardIndex {
    card_index("1bc44216-4e06-4f66-89b7-5c327004604e")
}

fn baxter_building() -> CardIndex {
    card_index("71bc69a5-7cec-4abd-b97d-13f8e1f9afac")
}

fn brotherhood_headquarters() -> CardIndex {
    card_index("0d3a06d5-5bb9-4733-a55b-9e2c75de6b6e")
}

fn castle_garenbrig() -> CardIndex {
    card_index("de75e5dd-8a52-406c-b55c-96d686885500")
}

fn castle_locthwain() -> CardIndex {
    card_index("be811e70-aaaa-41f3-bf9e-5d3f9f719b49")
}

fn contested_cliffs() -> CardIndex {
    card_index("b891a683-2ebc-4e9c-b402-5dd9c1b42b69")
}

fn contested_war_zone() -> CardIndex {
    card_index("ed73de2b-d7f4-48d9-9be2-aa9d111b7aa7")
}

fn dragon_cursed_halls() -> CardIndex {
    card_index("5be7a4d5-33b7-464b-8851-d4ad35302e62")
}

fn endless_sands() -> CardIndex {
    card_index("c4033d97-769f-4811-8b11-f85b8817b7a2")
}

fn flagstones_of_trokair() -> CardIndex {
    card_index("f73979bb-91a5-4388-b70b-0cd7a4e14291")
}

fn flamekin_village() -> CardIndex {
    card_index("34a1eb04-08f6-49d8-a1d1-b987a76bd8b1")
}

fn forge_of_heroes() -> CardIndex {
    card_index("77807103-bcd5-479f-bedd-f5d97aa6d3d2")
}

fn ghost_quarter() -> CardIndex {
    card_index("2ec4288e-34c6-4831-a2c0-ba1ca1d9d1dc")
}

fn great_arashin_city() -> CardIndex {
    card_index("f40f374b-acaf-459d-9ccd-b0b22d1a3f28")
}

fn guildmages_forum() -> CardIndex {
    card_index("ace6403d-9fac-4d0f-a6ea-eb2ff3da259d")
}

fn helios_one() -> CardIndex {
    card_index("cfb1a656-0bf1-484d-b099-33087914250b")
}

fn ifnir_deadlands() -> CardIndex {
    card_index("af698bd5-5f56-4d2a-9f02-8c3e781210cd")
}

fn iron_hills() -> CardIndex {
    card_index("a71e8d07-1a49-47a0-834e-de87d750a200")
}

fn maze_of_shadows() -> CardIndex {
    card_index("b7b51ab1-403e-4640-8827-b04965aa6760")
}

fn mines_of_moria() -> CardIndex {
    card_index("583cdebe-0195-45be-bd2e-5765f07cb902")
}

fn mirkwood() -> CardIndex {
    card_index("cd49aa99-bf84-4edd-aecc-6dae78b73412")
}

fn mistrise_village() -> CardIndex {
    card_index("339f5334-b65a-445a-a016-20e997e0b4bb")
}

fn murmuring_bosk() -> CardIndex {
    card_index("42b9d383-3fe2-4fc8-ab86-f80a288d502b")
}

fn mystifying_maze() -> CardIndex {
    card_index("58bd67a8-1833-4827-aa33-1c141568f481")
}

fn nivix_aerie_of_the_firemind() -> CardIndex {
    card_index("9c482f1d-08b4-4882-918c-448a556d3fbe")
}

fn nomad_stadium() -> CardIndex {
    card_index("4034bec6-e3c7-4d3f-81df-7c903977a606")
}

fn pit_of_offerings() -> CardIndex {
    card_index("044d2788-6daa-4849-a813-1f577eef9295")
}

fn public_thoroughfare() -> CardIndex {
    card_index("de5b995c-9691-4555-9070-66bcbc29f955")
}

fn ruins_of_oran_rief() -> CardIndex {
    card_index("7140f396-1bfa-4b28-ba28-fa15eba74652")
}

fn sejiri_steppe() -> CardIndex {
    card_index("3dfbf95e-a91b-429c-96e2-95ac777e7027")
}

fn shefet_dunes() -> CardIndex {
    card_index("8305715e-f711-47d6-8efe-d0efe4ced418")
}

fn skyline_cascade() -> CardIndex {
    card_index("79301ae1-8c9c-4723-be21-dc27e1646f35")
}

fn surtland_frostpyre() -> CardIndex {
    card_index("965aa666-3919-4053-8584-b773bdd54f0b")
}

fn the_mycosynth_gardens() -> CardIndex {
    card_index("03f5c566-825c-4c46-9c01-a2f9b1e70a13")
}

fn throne_of_makindi() -> CardIndex {
    card_index("7e8198e9-0f3b-420b-ab09-74f13f4fd548")
}

fn tolaria() -> CardIndex {
    card_index("9879a4f3-3b9c-45cf-af03-7f2ae4c689b4")
}

fn unclaimed_territory() -> CardIndex {
    card_index("584b15f2-6ae9-413a-8b8d-9244dbea4878")
}

fn dwarven_armorer() -> CardIndex {
    card_index("5bbd27b1-0afd-4d98-a73c-c348c8f08625")
}

fn hashep_oasis() -> CardIndex {
    card_index("eab70fff-6a9f-4f9f-89a2-b6910c199e46")
}

fn immersturm_skullcairn() -> CardIndex {
    card_index("354a7376-fb4b-424d-8964-93727302dccb")
}

fn lake_town() -> CardIndex {
    card_index("717c6beb-81c6-43ed-aab0-aedfc1cbac33")
}

fn mech_hangar() -> CardIndex {
    card_index("abc04775-171d-41f3-83ea-4b4eb72723d5")
}

fn smugglers_copter() -> CardIndex {
    card_index("49136bdc-bc50-49a2-999a-1ef9c16ea130")
}

fn mistveil_plains() -> CardIndex {
    card_index("bb5c1817-ac22-4779-9005-251bc354f181")
}

fn talon_gates_of_madara() -> CardIndex {
    card_index("8c45bf9d-a017-43bf-9e32-67810a8a217b")
}

fn turtle_lair() -> CardIndex {
    card_index("eb002bbc-08df-4bf0-bea3-46494ad261b6")
}

fn axgard_armory() -> CardIndex {
    card_index("bce30fd0-ed1e-495d-9149-6a4c81c45c7b")
}

fn cabaretti_courtyard() -> CardIndex {
    card_index("65424bea-fd53-4f85-9757-0b91a6d40ba4")
}

fn cavernous_maw() -> CardIndex {
    card_index("952ab8fe-f7d3-4673-89de-8c6d3f8a081f")
}

fn elven_passage() -> CardIndex {
    card_index("97a2cd39-6b54-496b-b3ac-dab9dfed7edc")
}

fn escape_tunnel() -> CardIndex {
    card_index("0056fc91-4398-471c-b561-7ff99750ac8a")
}

fn fire_nation_palace() -> CardIndex {
    card_index("f2000fb8-39c6-4ad6-a020-5245faaa1eba")
}

fn hobbit_hole() -> CardIndex {
    card_index("17492186-9814-4c41-8111-1f000a96c212")
}

fn realm_of_koh() -> CardIndex {
    card_index("bb9ce416-eef1-49e8-89a0-2b6837505070")
}

fn sunken_citadel() -> CardIndex {
    card_index("508189e1-9cef-4f9c-8ff1-078c99a0f603")
}

fn volatile_fault() -> CardIndex {
    card_index("95c44f28-f7fa-4785-83b9-0d81be0db0c8")
}

fn blinkmoth_nexus() -> CardIndex {
    card_index("40d45c02-6416-4e19-8fe3-0ddadf5ba627")
}

fn creeping_tar_pit() -> CardIndex {
    card_index("250cb58b-2924-4dff-92fe-ac0ebbbeb218")
}

fn faceless_haven() -> CardIndex {
    card_index("f74107d5-fb4a-464b-9251-42b84d91775d")
}

fn horizon_of_progress() -> CardIndex {
    card_index("59a82f57-fe2f-4834-a4ee-4b948eef1e12")
}

fn lake_of_the_dead() -> CardIndex {
    card_index("bdf476e5-1d57-4b17-b45b-d52fd75aadeb")
}

fn lotus_vale() -> CardIndex {
    card_index("01fc5bb3-ebd7-4ab4-8aef-2ece1e1d9b7c")
}

fn maestros_theater() -> CardIndex {
    card_index("9464ddf2-4bcb-44f6-b945-89a132544de6")
}

fn riveteers_overlook() -> CardIndex {
    card_index("5548ff43-e5f6-4a63-8562-a2b1de06d6f5")
}

fn sanctum_of_ugin() -> CardIndex {
    card_index("72cb5dcd-9b24-435c-921a-3766108374c4")
}

fn the_black_gate() -> CardIndex {
    card_index("40eb9904-dea3-47cf-963a-04821f98ba64")
}

fn evasive_action() -> CardIndex {
    card_index("4543a99d-eefa-470d-976d-11250524ae28")
}

fn gaea_s_might() -> CardIndex {
    card_index("73b26f12-78eb-4d01-9dd6-ee643c7a80a8")
}

// oracle_id = "1423b8e6-9165-4a89-a6ed-18085f460bca"
fn power_armor() -> CardIndex {
    card_index("1423b8e6-9165-4a89-a6ed-18085f460bca")
}

// oracle_id = "6789a170-f2c5-4fc0-8a45-2b2361e67410"
fn chord_of_calling() -> CardIndex {
    card_index("6789a170-f2c5-4fc0-8a45-2b2361e67410")
}

fn green_suns_zenith() -> CardIndex {
    card_index("0d96b60b-a060-48ee-bb83-93f1c4a10669")
}

fn midgar_city_of_mako() -> CardIndex {
    card_index("4e34a49d-f031-48ac-a458-97b79124b76c")
}

fn agatha_s_soul_cauldron() -> CardIndex {
    card_index("c259e16f-2a44-4552-8678-815f757a02e8")
}

fn wishclaw_talisman() -> CardIndex {
    card_index("81c70ae7-3c18-4c9b-8505-e4db9e0e6518")
}

fn dragonback_assault() -> CardIndex {
    card_index("413fb2db-f1a1-4d22-ac37-a52821d35ca2")
}

fn twists_and_turns() -> CardIndex {
    card_index("740aa9d9-91a9-431e-8bf9-1344e5273e27")
}

fn brokers_hideout() -> CardIndex {
    card_index("bd002797-a545-4bee-88bf-b878436e7cca")
}

fn basilisk_gate() -> CardIndex {
    card_index("8733a4fc-4068-4af4-9598-dc3d895e8556")
}

fn blighted_woodland() -> CardIndex {
    card_index("02679a2e-303d-412f-87d8-0a37a8ca259c")
}

fn cori_mountain_monastery() -> CardIndex {
    card_index("35c60b66-8c85-432e-90fe-99c19d21ed15")
}

fn arid_archway() -> CardIndex {
    card_index("3be3d7e6-7860-438a-b8c8-ef154c18c163")
}

fn scorched_ruins() -> CardIndex {
    card_index("6ee68855-c8c5-422b-88da-163c09a96416")
}

fn cryptic_spires() -> CardIndex {
    card_index("6d6a25fb-0432-4c7d-b0e6-e787ddc71218")
}

fn dakmor_salvage() -> CardIndex {
    card_index("cdc4048a-73ec-4ec1-a179-2b36c397bf1a")
}

fn shizo_death_s_storehouse() -> CardIndex {
    card_index("008f2698-1721-45a3-8353-10f2f400dc8f")
}

fn spinerock_knoll() -> CardIndex {
    card_index("690c7f8e-fea2-4920-afa7-02ff120701a1")
}

fn vesuva() -> CardIndex {
    card_index("4001b868-ada1-43f4-92e2-27ab0e80c913")
}

fn obscura_storefront() -> CardIndex {
    card_index("dc31a6f8-6228-4a25-b937-5d8d78514333")
}

fn barad_dur() -> CardIndex {
    card_index("88159872-d37d-4847-b048-e4a9af6437bd")
}

fn jidoor_aristocratic_capital() -> CardIndex {
    card_index("bd513d9d-5aa2-4860-bd86-8b5d9430f133")
}

fn dungeon_descent() -> CardIndex {
    card_index("f086a63c-0c62-4674-bd27-82e7aed12b1a")
}

fn howltooth_hollow() -> CardIndex {
    card_index("463fc699-f4fc-4112-a6b3-6dcb642203e6")
}

fn grove_of_the_guardian() -> CardIndex {
    card_index("f746612a-fbed-44ca-b2cc-5928e10cf4bb")
}

fn collector_ouphe() -> CardIndex {
    card_index("0c4bc9ea-a5fd-4f44-96a1-5448eee228c4")
}

fn reshape() -> CardIndex {
    card_index("42a3855d-25ab-45b3-9e5d-9a0f3da35a05")
}

fn finale_of_devastation() -> CardIndex {
    card_index("69872a9a-fe54-4e58-940c-89395af71acd")
}

fn whir_of_invention() -> CardIndex {
    card_index("152b91c9-cc07-4ca8-944f-9bc2242a2283")
}

fn land_cap() -> CardIndex {
    card_index("bfec4d0a-3792-4bc3-bae1-e639da5bb9a6")
}

fn archdruid_s_charm() -> CardIndex {
    card_index("3c1ef404-e2c6-486d-a5a2-d5779c71d498")
}

fn walk_in_closet() -> CardIndex {
    card_index("52e77cc3-f8e9-4a20-811b-fe1e46a96ad7")
}

fn legion_s_landing() -> CardIndex {
    card_index("f7d8b91b-6541-4d3e-af51-7e000eac69c1")
}

fn mizzix_s_mastery() -> CardIndex {
    card_index("40362fe0-a1a9-4d76-8c35-eac474b91af5")
}

fn ojer_pakpatiq_deepest_epoch() -> CardIndex {
    card_index("34ef174e-1b3d-43d5-9f72-3d35befbdd7f")
}

fn fatehold_chronologist() -> CardIndex {
    card_index("1063822f-47d3-42e9-8a21-f62b12609fe1")
}

fn assassin_s_trophy() -> CardIndex {
    card_index("ac10d218-f9a6-4058-9cda-a15ca1b0b7b5")
}

fn beyeen_veil() -> CardIndex {
    card_index("b03de49d-246f-44e2-9487-9e4e43ec7be4")
}

fn fire() -> CardIndex {
    card_index("ae92942b-919c-4ea9-b693-85fcef765d5a")
}

fn ghoul_s_feast() -> CardIndex {
    card_index("042e0533-faf4-475e-be7b-438d23c6e605")
}

fn jwari_disruption() -> CardIndex {
    card_index("941a4b14-ea2a-4bd0-8cc2-d609f80df32c")
}

fn kabira_takedown() -> CardIndex {
    card_index("0bb73c07-0220-4ba9-8d85-3c357c223833")
}

fn legion_leadership() -> CardIndex {
    card_index("ad225ec2-ff3a-48f6-81a7-dfdd1b75e1f7")
}

fn lose_focus() -> CardIndex {
    card_index("1cea6439-7ae5-4887-8c33-7da9fb36e2d4")
}

fn razorgrass_ambush() -> CardIndex {
    card_index("5da954fa-9001-4557-825c-1462035d21ed")
}

fn sejiri_shelter() -> CardIndex {
    card_index("d54e4e37-042b-44a5-918d-757308545d4d")
}

fn tear_asunder() -> CardIndex {
    card_index("610af0f7-b5e3-43fb-9d02-7c59bd99034c")
}

fn fell_the_profane() -> CardIndex {
    card_index("053a69d8-2b5e-4f14-8b02-ca405891dc4a")
}

fn hagra_mauling() -> CardIndex {
    card_index("37783ce6-af58-4ef6-8ab4-587079970307")
}

fn inner_calm_outer_strength() -> CardIndex {
    card_index("b7bdbae5-549f-403b-83ca-4f9a1ac93e93")
}

fn kazuul_s_fury() -> CardIndex {
    card_index("f8410804-632b-4f18-9a73-6dccc7e4582d")
}

fn khalni_ambush() -> CardIndex {
    card_index("37a55560-6e32-4f54-b9a8-fd157aea6eb5")
}

fn krosan_grip() -> CardIndex {
    card_index("3e39224c-72ce-4ecc-aa17-12c071ea1f3e")
}

fn rush_of_inspiration() -> CardIndex {
    card_index("bbd569cc-bc21-46df-b8eb-5b5bcd8fe762")
}

fn silundi_vision() -> CardIndex {
    card_index("b0182ca0-f353-4012-9121-6f4ac9f7a046")
}

fn sink_into_stupor() -> CardIndex {
    card_index("bcc6eece-75ea-494c-b33a-d4477d504e0b")
}

fn sultai_charm() -> CardIndex {
    card_index("46ed38d1-e642-4cea-99ed-a9c17fd982b1")
}

fn valakut_awakening() -> CardIndex {
    card_index("ff0ab867-b710-4b1a-baed-95fc3cf68f79")
}

fn agadeem_s_awakening() -> CardIndex {
    card_index("562d71b9-1646-474e-9293-55da6947a758")
}

fn bala_ged_recovery() -> CardIndex {
    card_index("d2075f58-b0e9-4e85-b7e6-0523a27a1d5b")
}

fn bridgeworks_battle() -> CardIndex {
    card_index("9d581188-ce80-494e-bd38-f411e1f4efb5")
}

fn maelstrom_pulse() -> CardIndex {
    card_index("95ce305f-34bc-4d6d-b7ba-ffd4b2a25336")
}

fn makindi_stampede() -> CardIndex {
    card_index("342e08f9-d4d0-4408-8621-66e087058616")
}

fn pelakka_predation() -> CardIndex {
    card_index("b0fd6889-20b4-439b-aa97-2e90aca1675a")
}

fn stump_stomp() -> CardIndex {
    card_index("eb7b1284-0b2c-4b6a-a389-b2b932838083")
}

fn sundering_eruption() -> CardIndex {
    card_index("c95309e9-5c2f-4518-b2fd-825d3d0a4ae0")
}

fn thoughtseize() -> CardIndex {
    card_index("edd8d1e8-be43-4c38-bb3a-83081fbaf0b5")
}

fn waterlogged_teachings() -> CardIndex {
    card_index("e6ad1be9-f13d-4590-b3db-e2d0fff46f03")
}

fn yawgmoth_s_will() -> CardIndex {
    card_index("322f0459-f394-44f0-977b-55fd0cbe0712")
}

fn dowsing_dagger() -> CardIndex {
    card_index("df34a6ad-ae1c-4470-8c9e-49815bba1973")
}

fn dowsing_device() -> CardIndex {
    card_index("2f4374f6-c695-4a5d-a6d6-0e41eaa587ca")
}

fn emeria_s_call() -> CardIndex {
    card_index("6ec2a242-9068-4ee2-8ac8-8341cc570f56")
}

fn ondu_inversion() -> CardIndex {
    card_index("15fc4e74-300e-4c2d-8ed7-004553b2f7c2")
}

fn sea_gate_restoration() -> CardIndex {
    card_index("4a8d41fe-e04d-484b-a7d1-19be311e6ca7")
}

fn song_mad_treachery() -> CardIndex {
    card_index("81b61770-2ed5-4a50-84d0-97790002fc5a")
}

fn suppression_ray() -> CardIndex {
    card_index("b592568b-11b0-4081-90a7-30cfb9c1ba80")
}

fn tarrian_s_journal() -> CardIndex {
    card_index("a75b02ba-b0c8-47e3-a05c-e9ba221a7578")
}

fn thaumatic_compass() -> CardIndex {
    card_index("f9085e55-2833-41b7-9100-a35dc04dee93")
}

fn treasure_map() -> CardIndex {
    card_index("0b55eac6-a745-4bf4-8926-5ce83bc38d7d")
}

fn zof_consumption() -> CardIndex {
    card_index("d9f11985-e460-425d-b083-9cb0edf1983a")
}

fn aclazotz_deepest_betrayal() -> CardIndex {
    card_index("fcdfe9d5-2743-4d3e-ab57-bf0f96beaa15")
}

fn brass_s_tunnel_grinder() -> CardIndex {
    card_index("af1553eb-4f9f-4335-9078-56649bd8d8fc")
}

fn conqueror_s_galleon() -> CardIndex {
    card_index("88b18901-50cd-461c-b1bc-be900210be8e")
}

fn fanatic_of_rhonas() -> CardIndex {
    card_index("7973820b-fdaf-46ec-9e3e-d4c0e77b5067")
}

fn matzalantli_the_great_door() -> CardIndex {
    card_index("16182e01-22ff-4786-985d-919b47c4aa4d")
}

fn nissa_resurgent_animist() -> CardIndex {
    card_index("c1fc5923-c3cd-448a-98d1-c154661c2812")
}

fn ojer_axonil_deepest_might() -> CardIndex {
    card_index("d3b7b541-6f05-46c1-8031-c848c4bd4635")
}

fn ojer_taq_deepest_foundation() -> CardIndex {
    card_index("486bb9a5-73f1-4cec-b097-fb07ac80b72e")
}

fn primal_amulet() -> CardIndex {
    card_index("8e4d0da0-c7d8-4a20-9bfd-02c1331a7a49")
}

fn the_one_ring() -> CardIndex {
    card_index("3aa83ed2-f48b-4ce6-a614-2c54ddf50538")
}

fn thousand_moons_smithy() -> CardIndex {
    card_index("32af5f7b-a970-484a-9aff-226749551d32")
}

fn druid_class() -> CardIndex {
    card_index("dcbcbf42-4654-487a-acad-21f2606d229b")
}

fn garruk_s_uprising() -> CardIndex {
    card_index("3127ae9b-a7a7-43ec-89d7-688f8445b33d")
}

fn grasping_shadows() -> CardIndex {
    card_index("522a4b02-24c7-45d2-9097-2803cc9fffad")
}

fn growing_rites_of_itlimoc() -> CardIndex {
    card_index("ea9c459a-6047-43aa-968f-a582be4000e8")
}

fn hadana_s_climb() -> CardIndex {
    card_index("93b91d18-6acf-42e5-9a31-bc6e01f90c1f")
}

fn journey_to_eternity() -> CardIndex {
    card_index("7d6ccd0b-df16-40b2-930b-bcde0b6ef73f")
}

fn path_of_mettle() -> CardIndex {
    card_index("db9ea3f9-c723-422f-98cc-a3ef7ca2c290")
}

fn search_for_azcanta() -> CardIndex {
    card_index("f74c4d96-bc4a-4d32-9519-a753d192144e")
}

fn sidequest_catch_a_fish() -> CardIndex {
    card_index("bd7c328e-0380-46f8-bb85-7bf4e201b7ac")
}

fn storm_the_vault() -> CardIndex {
    card_index("72205fac-a94a-45cc-94c6-40ece2fdce0e")
}

fn vance_s_blasting_cannons() -> CardIndex {
    card_index("5e7eca9c-a7b8-4b7b-a0a0-e8937530145a")
}

fn bloodsoaked_insight() -> CardIndex {
    card_index("c52fc8a1-43c6-41f8-b010-03be7c89ef1d")
}

fn fable_of_the_mirror_breaker() -> CardIndex {
    card_index("c0957e5e-c71b-439c-931c-9f55d2f76ace")
}

fn grist_the_hunger_tide() -> CardIndex {
    card_index("0efb0d7e-dea0-4817-a243-15066e9ef333")
}

fn oko_thief_of_crowns() -> CardIndex {
    card_index("60c60923-ff1b-43f7-8768-731499fcffc9")
}

fn shatterskull_smashing() -> CardIndex {
    card_index("78301998-fd9b-4cd5-afad-dbcb43cac2a7")
}

fn turntimber_symbiosis() -> CardIndex {
    card_index("403b59f3-7ade-4bc2-a3e6-de0c3c700f18")
}

fn welcome_to() -> CardIndex {
    card_index("a4b37d16-95b3-4143-a0b2-ad9f2aba91f8")
}

fn wrenn_and_realmbreaker() -> CardIndex {
    card_index("4566fb92-448e-4b3f-9045-9d74323c35d1")
}

fn jayemdae_tome() -> CardIndex {
    card_index("39ee576a-0803-4063-9c84-f2b537e4d44c")
}

fn rod_of_ruin() -> CardIndex {
    card_index("c29a04ab-4e14-45d8-a993-3fffd0e2f6eb")
}

fn sisay_s_ring() -> CardIndex {
    card_index("8f5822ae-651f-410e-9316-5522eeb52d72")
}

fn thran_dynamo() -> CardIndex {
    card_index("a699c663-8131-4045-9265-a83e86609374")
}

fn tower_of_champions() -> CardIndex {
    card_index("f7880784-e1dc-4250-812e-eda41fe91e36")
}

fn tower_of_fortunes() -> CardIndex {
    card_index("dfb23df9-f912-4daa-9842-01bd12b4a72a")
}

fn ambush_party() -> CardIndex {
    card_index("454ed336-362d-46d9-96d5-8fcaa8a1c543")
}

fn angel_of_mercy() -> CardIndex {
    card_index("a2daaf32-dbfe-4618-892e-0da24f63a44a")
}

fn benalish_knight() -> CardIndex {
    card_index("32b401e9-163f-4917-a728-fc63b25ef602")
}

fn briarknit_kami() -> CardIndex {
    card_index("913dcf97-0804-4b21-8daa-f2b481dc0f2f")
}

fn crowd_favorites() -> CardIndex {
    card_index("1ead750f-14a6-4f25-9eb8-9472c2fdac35")
}

fn deadly_insect() -> CardIndex {
    card_index("98273b0d-2b41-486b-9498-30a692c03982")
}

fn fire_snake() -> CardIndex {
    card_index("e96542ed-1931-4da1-9d9e-d10878c4ae6b")
}

fn iron_tusk_elephant() -> CardIndex {
    card_index("a299470a-72ea-4848-833b-dc894db7be15")
}

fn juzam_djinn() -> CardIndex {
    card_index("4e81596c-9225-43d1-bd35-798212144f2c")
}

fn killer_whale() -> CardIndex {
    card_index("5ace3817-5a98-4eee-b6b9-8c105cea302f")
}

fn lightning_elemental() -> CardIndex {
    card_index("58aee5cb-7b88-446e-ab10-9f83c10d7227")
}

fn nettletooth_djinn() -> CardIndex {
    card_index("f1d300b6-f9cf-40a9-8520-d78cd7d813cf")
}

fn norwood_archers() -> CardIndex {
    card_index("4e4e96aa-2f05-4f1d-96f8-6c42cd3be589")
}

fn pardic_collaborator() -> CardIndex {
    card_index("9b06cfae-1655-4b0e-a160-9b32f27c7c9a")
}

fn pendrell_drake() -> CardIndex {
    card_index("8d8981df-4cdc-4fd4-acd8-095fa3281281")
}

fn ramirez_de_pietro() -> CardIndex {
    card_index("48b9c510-03ed-41cb-8c17-16110d447490")
}

fn seahunter() -> CardIndex {
    card_index("59e1899f-a9e8-48aa-b3fb-0b7d9fd6859c")
}

fn shock_troops() -> CardIndex {
    card_index("fdf2a8da-2933-44c5-b483-f035144da752")
}

fn shu_soldier_farmers() -> CardIndex {
    card_index("174cf7cd-3e8c-4f90-abc1-a78a0ce832d2")
}

fn slinking_skirge() -> CardIndex {
    card_index("82875793-b264-4ceb-8525-ef3b6d086072")
}

fn sliver_queen() -> CardIndex {
    card_index("b8376cca-ea96-478a-8e98-c4482031300a")
}

fn spotted_griffin() -> CardIndex {
    card_index("4916773d-5ccb-48ff-8aa3-09771ae88e81")
}

fn steelshaper_apprentice() -> CardIndex {
    card_index("bf325b3a-0b28-4660-8b51-4334b59a9034")
}

fn talruum_minotaur() -> CardIndex {
    card_index("c5661500-c48f-4f6d-bbe7-c8bd7118d862")
}

fn tuknir_deathlock() -> CardIndex {
    card_index("0b6fa658-a344-4202-866b-e19997e32ff2")
}

fn vulshok_berserker() -> CardIndex {
    card_index("694f7e51-7b8b-4f00-bd77-a52eded4aaa1")
}

fn wirewood_channeler() -> CardIndex {
    card_index("8badfd34-9b63-4ada-8012-b8cddaf5b492")
}

fn zuberi_golden_feather() -> CardIndex {
    card_index("5a43b164-61a4-4dcb-9397-45356ad4260e")
}

fn mental_discipline() -> CardIndex {
    card_index("b22080d6-a9ed-4bdd-a604-058e0e3e9463")
}

fn night_of_souls_betrayal() -> CardIndex {
    card_index("916bd025-c44f-49c9-8d76-4b7b2f9a8ba3")
}

fn overgrown_estate() -> CardIndex {
    card_index("4d52c4a5-e5c8-4fb4-be50-78d5482dd1ae")
}

fn inspiration() -> CardIndex {
    card_index("8f32ceb2-92c2-4dde-bf73-40bb79c3fcef")
}

fn lightning_blast() -> CardIndex {
    card_index("91fd731c-e076-4f2d-9f22-872880c3cc3d")
}

fn searing_wind() -> CardIndex {
    card_index("b196045d-ece1-46ac-a647-b65f04225c10")
}

fn adventurers_inn() -> CardIndex {
    card_index("232bd88c-ecdb-43dd-b34a-d381cb3bedf2")
}

fn griffin_canyon() -> CardIndex {
    card_index("ba642c8b-9ade-4501-8393-672fd53d4955")
}

fn memorial_to_genius() -> CardIndex {
    card_index("81763d7d-3897-4be9-bbf6-f6f5dee366ff")
}

fn memorial_to_war() -> CardIndex {
    card_index("f98db69c-b330-4560-ac53-10857674466b")
}

fn racers_ring() -> CardIndex {
    card_index("7e2eb4d5-22a3-43c1-8cf3-e85723da1b61")
}

fn radiant_fountain() -> CardIndex {
    card_index("6db442e5-fbcc-4456-a4c5-bea1aee3fc8e")
}

fn yavimaya_cradle_of_growth() -> CardIndex {
    card_index("8dd5f5af-d2d8-4356-8617-8381081b930c")
}

fn break_asunder() -> CardIndex {
    card_index("d2c53737-c265-46e3-a779-52c6b4f82d7d")
}

fn brilliant_plan() -> CardIndex {
    card_index("d83e1a42-11d1-412a-b66c-850c7a528777")
}

fn icatian_town() -> CardIndex {
    card_index("aa8b60b6-cf55-4aaa-9caa-2b17942d8269")
}

fn soul_feast() -> CardIndex {
    card_index("8186fd80-015f-470c-9e1c-cbf45764a057")
}

fn spoils_of_victory() -> CardIndex {
    card_index("852bd598-6e48-43c8-9211-740ae9e0c42e")
}

fn tidings() -> CardIndex {
    card_index("72897780-094d-4a21-8b1c-419a9defd2fb")
}

fn unyaro_bee_sting() -> CardIndex {
    card_index("a500313b-35e3-4ebc-9144-e9486784757b")
}

fn fodder_cannon() -> CardIndex {
    card_index("aaf171bd-a4bb-4ce4-836a-da193c94f42e")
}

fn skull_catapult() -> CardIndex {
    card_index("eef931d8-4048-4c33-bd8c-0f67d1083ee6")
}

fn tower_of_eons() -> CardIndex {
    card_index("74a71bbd-c307-481c-b0cf-70e24b0c8ad4")
}

fn ur_golem_s_eye() -> CardIndex {
    card_index("fb34fc00-e60d-41fc-9393-ca4248ec0a1c")
}

fn air_elemental() -> CardIndex {
    card_index("7744bae4-a8b7-44a5-9b4c-0048ad4cc448")
}

fn aven_brigadier() -> CardIndex {
    card_index("910ae0ca-257d-4b45-b039-f26e7b2f3d5c")
}

fn azami_lady_of_scrolls() -> CardIndex {
    card_index("0f8b97fe-3e5e-47c2-9a9d-7f77482aa159")
}

fn boa_constrictor() -> CardIndex {
    card_index("f0ab6ca6-098a-416a-89b2-9a93c9d0e60b")
}

fn deathcurse_ogre() -> CardIndex {
    card_index("0d40bc98-28e7-4fa5-b848-1cbac5345c12")
}

fn flowstone_wyvern() -> CardIndex {
    card_index("13459bf3-fcdc-4838-9669-918f97113054")
}

fn giant_warthog() -> CardIndex {
    card_index("0eb41c36-5910-41fd-97fc-1cae2332e5c5")
}

fn harmattan_efreet() -> CardIndex {
    card_index("84b1d4e1-ca8a-4eed-b5c4-364876938aee")
}

fn jhovall_rider() -> CardIndex {
    card_index("84750c9a-ddf7-4ee1-b93b-3dc9ab73e4d5")
}

fn kodama_of_the_north_tree() -> CardIndex {
    card_index("15d8f129-2518-45f4-9f34-a4fcd4d859af")
}

fn plated_spider() -> CardIndex {
    card_index("15163c99-9388-4225-b707-4e057b2adfbc")
}

fn sire_of_the_storm() -> CardIndex {
    card_index("68f1db12-82fb-4bf1-908d-db38fd67efe5")
}

fn spiritual_guardian() -> CardIndex {
    card_index("d03860b4-c663-4230-9200-6b89fa849ae7")
}

fn striped_bears() -> CardIndex {
    card_index("ac856e91-1b88-464b-9420-dc311ce814ea")
}

fn thriss_nantuko_primus() -> CardIndex {
    card_index("19ed8616-5d27-4f21-88a9-2af7cf5034ca")
}

fn trenching_steed() -> CardIndex {
    card_index("31136103-535a-4d11-819a-1cbd57ad48a4")
}

fn field_of_souls() -> CardIndex {
    card_index("4d7a5b14-8fce-41f2-a0d5-fff3d15f41f6")
}

fn altars_light() -> CardIndex {
    card_index("fa9b6be2-b88c-4302-b7e2-faf25a60bcb9")
}

fn might_of_oaks() -> CardIndex {
    card_index("8331f281-819b-4a0b-bad7-bd86dbedb877")
}

fn opportunity() -> CardIndex {
    card_index("1f544a9f-c238-4858-be19-d6cd7d023dcc")
}

fn encroaching_wastes() -> CardIndex {
    card_index("43144f06-079b-4515-a03a-01ea3e90d586")
}

fn guadosalam_farplane_gateway() -> CardIndex {
    card_index("849c97e4-df15-4ecb-bdf8-283bb497d90c")
}

fn looming_spires() -> CardIndex {
    card_index("7d09b136-525f-49dd-a3a2-dfaca4e8e9a8")
}

fn misty_palms_oasis() -> CardIndex {
    card_index("dfd2c57a-4557-4df1-8f6f-da2cbd317f12")
}

fn serpent_s_pass() -> CardIndex {
    card_index("f715f701-a735-42ab-b31a-8e1bd04ac5ff")
}

fn skarrg_the_rage_pits() -> CardIndex {
    card_index("92bac34e-2045-4331-842f-185711c1ac56")
}

fn tramway_station() -> CardIndex {
    card_index("e90e519d-023e-4d19-85ad-9972a76df3ba")
}

fn white_lotus_hideout() -> CardIndex {
    card_index("2cdbfda3-98fc-4108-b551-c7049168924e")
}

fn diabolic_tutor() -> CardIndex {
    card_index("14589b6b-1814-46f9-a364-83cc15dacac2")
}

fn vampiric_feast() -> CardIndex {
    card_index("1980ca2e-a415-4de1-ac30-7055507e82a2")
}

fn jedit_s_dragoons() -> CardIndex {
    card_index("3f6df152-a0b9-441a-a8aa-8bb77f70d491")
}

fn mana_geyser() -> CardIndex {
    card_index("a8dba58b-2956-492e-ae30-49db2ae68e53")
}

fn the_hive() -> CardIndex {
    card_index("87a77482-f286-4fb1-b179-85b2095bb768")
}

fn tower_of_murmurs() -> CardIndex {
    card_index("c383b28c-b319-49cd-acf1-4721a1301bbb")
}

fn border_patrol() -> CardIndex {
    card_index("fa5e2575-51bb-448e-9aaf-0e0f5956ac98")
}

fn djinn_of_the_lamp() -> CardIndex {
    card_index("72b42c63-fe4d-4823-9692-30fb5bab384a")
}

fn firescreamer() -> CardIndex {
    card_index("b86c0c23-bb29-4af0-bbd4-50b3ae634375")
}

fn flame_spirit() -> CardIndex {
    card_index("123bfa86-4265-448f-97cd-2c0614212862")
}

fn flowstone_charger() -> CardIndex {
    card_index("a19e0a7f-d356-4d2c-8414-70a40ce18674")
}

fn flowstone_crusher() -> CardIndex {
    card_index("cc18fcfc-bfe8-41d8-9037-5ff05b2a9c44")
}

fn goblin_commando() -> CardIndex {
    card_index("76c02534-35e3-4950-b4b3-90c679cdf6a7")
}

fn halberdier() -> CardIndex {
    card_index("1c3c23b5-b771-4117-83b5-febf7e96281a")
}

fn joven() -> CardIndex {
    card_index("9e767d44-feff-449a-bed9-0866c7bce846")
}

fn kami_of_tattered_shoji() -> CardIndex {
    card_index("a7cb1da6-e56a-42b6-8087-cf822783373f")
}

fn kavu_climber() -> CardIndex {
    card_index("91320afd-1d42-4cb5-ae40-4eed2fa91dfe")
}

fn kavu_mauler() -> CardIndex {
    card_index("6ffbcaba-5437-4fb6-a2d6-e94b1e6dc1d2")
}

fn keening_banshee() -> CardIndex {
    card_index("56b69e46-a1e4-4545-99e2-ee4385b7e429")
}

fn lava_hounds() -> CardIndex {
    card_index("86c0dc85-d146-4dfa-819e-f78835bfffff")
}

fn lithophage() -> CardIndex {
    card_index("b4eb3d7e-a234-4ed5-8611-fcb818686fcf")
}

fn macetail_hystrodon() -> CardIndex {
    card_index("2f1e2742-d7df-4893-abc8-cb927c500569")
}

fn megatog() -> CardIndex {
    card_index("bde427fa-8a00-4eff-9371-81324bc75364")
}

fn moorish_cavalry() -> CardIndex {
    card_index("bccdc42f-9e55-4f8e-a29e-ef39f38add6a")
}

fn phyrexian_plaguelord() -> CardIndex {
    card_index("aff9e844-9e03-490b-b44f-10d385738cc6")
}

fn plated_slagwurm() -> CardIndex {
    card_index("4aec7624-e406-45a4-b2b6-2e8d29f6268a")
}

fn riven_turnbull() -> CardIndex {
    card_index("2742d506-897f-4d30-ba43-ce0374984849")
}

fn rummaging_wizard() -> CardIndex {
    card_index("99002f0c-762b-4714-afe3-5b011350aefd")
}

fn skirge_familiar() -> CardIndex {
    card_index("ba95f24d-42da-48ce-bcf1-1b7c4b3c45b5")
}

fn skyhunter_patrol() -> CardIndex {
    card_index("aadcff6f-9207-4d90-a12d-4913c96867e2")
}

fn skyshroud_poacher() -> CardIndex {
    card_index("4f5921c1-b932-4d6b-bb8e-01992578abdc")
}

fn snapping_drake() -> CardIndex {
    card_index("e15060c3-3773-4548-8747-ff59dcf2b519")
}

fn trench_wurm() -> CardIndex {
    card_index("9cf65178-6408-46da-a940-f5cb0960b61f")
}

fn verdant_force() -> CardIndex {
    card_index("7a21ea22-3cd7-4c11-8895-5943c0d93a0d")
}

fn vigilant_drake() -> CardIndex {
    card_index("4f5eaf5a-dd52-4d48-91b5-d48763d13159")
}

fn wind_spirit() -> CardIndex {
    card_index("d4c22b68-c4cb-447a-97df-431e2f1e31f2")
}

fn zephid() -> CardIndex {
    card_index("88aa710c-26ed-490d-9a4b-4a2b48df1733")
}

fn castle() -> CardIndex {
    card_index("f3179c3c-7e53-44d2-b579-b9e677efe9d9")
}

fn embargo() -> CardIndex {
    card_index("edadd0bf-15c8-4e9c-810e-2edfd77a9d01")
}

fn narcissism() -> CardIndex {
    card_index("59cd56d2-42f3-44de-b1b8-a5f9105dcaa8")
}

fn noble_steeds() -> CardIndex {
    card_index("d6005e47-d545-4c16-b54a-bfe1dde61a98")
}

fn opposition() -> CardIndex {
    card_index("bf0b252d-1295-44e4-bac3-113b5732a2a9")
}

fn phyrexian_arena() -> CardIndex {
    card_index("ee579a32-a048-4335-b966-231ba731cdea")
}

fn spiritual_asylum() -> CardIndex {
    card_index("91924536-7a2b-44ec-9835-2efa402c83f9")
}

fn regress() -> CardIndex {
    card_index("2fa763ad-4d94-4c3a-a099-13ac22c09ce4")
}

fn volcanic_geyser() -> CardIndex {
    card_index("846a4f9c-d955-403f-8a08-5c7c3d32e180")
}

fn zap() -> CardIndex {
    card_index("56115482-3fd4-45fb-b800-be68c5509cf2")
}

fn baron_airship_kingdom() -> CardIndex {
    card_index("cc710da0-5a2e-4bc4-8fdd-d90e7bc1f224")
}

fn duskmantle_house_of_shadow() -> CardIndex {
    card_index("67b2cd0c-ecc8-4129-b1ac-820c9924190c")
}

fn orzhova_the_church_of_deals() -> CardIndex {
    card_index("8551a9cf-c54b-42d4-92d6-550f4890a3d7")
}

fn sandstone_bridge() -> CardIndex {
    card_index("08911e8e-cd67-4960-a927-958c33632469")
}

fn savai_triome() -> CardIndex {
    card_index("00625242-9348-4ef4-b975-f2ac82fee21d")
}

fn soaring_seacliff() -> CardIndex {
    card_index("a37544b6-0048-4213-8e40-76ba8a0b6d1b")
}

fn spara_s_headquarters() -> CardIndex {
    card_index("3123ec89-8e95-4761-ba17-747ec667509f")
}

fn vector_imperial_capital() -> CardIndex {
    card_index("8dcba63c-4701-4dc9-81f5-ca8e8933a3ba")
}

fn windurst_federation_center() -> CardIndex {
    card_index("e198126a-f280-47f7-8bd0-3dc9f5ff05a0")
}

fn desert_twister() -> CardIndex {
    card_index("6f880348-6dc8-4cf8-9313-9893c41a70a3")
}

fn ice_storm() -> CardIndex {
    card_index("a0b97e33-2d0c-4800-a9f5-9cd9be651ea8")
}

fn screaming_fury() -> CardIndex {
    card_index("eaded717-e0e2-4def-be7e-6bfb137628c2")
}

fn vengeance() -> CardIndex {
    card_index("1d001145-5d14-43a9-bf3b-3ce5c20b2a46")
}

fn dread_reaper() -> CardIndex {
    card_index("bd73ab86-0ac9-4ce0-be41-f4ad257e74f6")
}

fn furnace_whelp() -> CardIndex {
    card_index("8422f1e0-00ca-4ffb-a6b2-e2c9f96d7f23")
}

fn giant_crab() -> CardIndex {
    card_index("780f029f-e2e1-431b-9a8b-a1af51d7da77")
}

fn goblin_berserker() -> CardIndex {
    card_index("5f71f205-9551-416a-bba6-55bc65d11c85")
}

fn krosan_groundshaker() -> CardIndex {
    card_index("1300dc30-a089-41f5-b1a7-e4779ebb73ef")
}

fn ogre_berserker() -> CardIndex {
    card_index("9e6f76dc-b7db-49b6-a61e-3495b81e61d7")
}

fn pavel_maliki() -> CardIndex {
    card_index("41026872-3c7b-40c7-8d0b-9d2f9b6c3e91")
}

fn ravenous_baloth() -> CardIndex {
    card_index("ee771e66-72f8-480f-9920-92c68ab93c3b")
}

fn sabertooth_wyvern() -> CardIndex {
    card_index("acaae632-3203-4867-ad03-32619c3fcfe6")
}

fn serra_angel() -> CardIndex {
    card_index("4b7ac066-e5c7-43e6-9e7e-2739b24a905d")
}

fn silver_erne() -> CardIndex {
    card_index("6ca90451-861d-4aa8-93e7-3f3e02385530")
}

fn storm_spirit() -> CardIndex {
    card_index("6a413d09-dd67-4c3e-b81b-7bfe8b864dc9")
}

fn vedalken_entrancer() -> CardIndex {
    card_index("04cc5292-a485-4bb8-a143-9364229d480f")
}

fn infernal_tribute() -> CardIndex {
    card_index("a8bf79d2-29fd-4d2f-ac07-2e61d3886bc0")
}

fn seismic_assault() -> CardIndex {
    card_index("8ad4f2fe-6d98-4279-a331-3817d40ae46d")
}

fn mystic_denial() -> CardIndex {
    card_index("d2bd23a6-4f77-4d6e-bf8f-339cb7a4184d")
}

fn pull_under() -> CardIndex {
    card_index("b99ddb27-a59a-4cd3-88b1-97c759ddbd93")
}

fn smash() -> CardIndex {
    card_index("1602f8c7-fe00-4bc9-b4fc-b26b7223ac75")
}

fn verdigris() -> CardIndex {
    card_index("fb41ea1d-3491-4193-a390-8eb202fbae12")
}

fn ally_encampment() -> CardIndex {
    card_index("9d293b69-12b7-4b50-a0a7-c4f493dee30b")
}

fn okina_temple_to_the_grandfathers() -> CardIndex {
    card_index("3ad69bfb-2e51-4fe9-8d2f-7d071a4f1c69")
}

fn teetering_peaks() -> CardIndex {
    card_index("4a9437a6-4e61-48b6-8194-1c6ba6432250")
}

fn woodland_stream() -> CardIndex {
    card_index("e887fb3f-d4c9-4022-8f75-1de6ec94af96")
}

fn zhalfirin_void() -> CardIndex {
    card_index("13aab4fc-4c89-45e6-8275-b074b00d0ee9")
}

fn ziatora_s_proving_ground() -> CardIndex {
    card_index("f7e7b78c-c769-4720-8585-1874773eb342")
}

fn ancient_craving() -> CardIndex {
    card_index("78725353-9274-420a-b722-add0f43c444e")
}

fn bee_sting() -> CardIndex {
    card_index("f637d525-2f29-488a-9269-8e5aa377fbb7")
}

fn lava_flow() -> CardIndex {
    card_index("91c0a76e-3992-437f-b85a-97b0b4adbb84")
}

fn natural_spring() -> CardIndex {
    card_index("f7571a2e-aaf3-4148-ab76-2a2e35273c70")
}

fn touch_of_brilliance() -> CardIndex {
    card_index("6365aba1-78d3-416c-89cd-9449578eedbf")
}

fn princess_lucrezia() -> CardIndex {
    card_index("dd1d21f5-f5cd-43e4-8155-3cca85da80a9")
}

fn sharlayan_nation_of_scholars() -> CardIndex {
    card_index("564bdbdd-8392-4ee1-a132-1a17a67b2110")
}
