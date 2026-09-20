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
//! `tap_all_mana`, `library_size` — stays in this file. That is not tidiness
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

use crate::choice::{CastModeKind, YesNoPrompt};
use crate::object::Status;
use crate::zone::{Zone, ZoneLocation};
use baylee_cards_dsl::{CounterKind, KeywordSet};
use baylee_core::ids::{CardIndex, Defender, ObjectId};
use baylee_core::mana::ManaColor;
use baylee_core::types::TypeSet;

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

fn aurochs() -> CardIndex {
    card_index("3961ef7c-4eb4-482e-9cda-d49d6a29c5a9")
}

fn rootbreaker_wurm() -> CardIndex {
    card_index("d3edbb47-6892-4853-badc-cc01499d4e55")
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

/// Taps everything that makes mana for `seat`, which is what a player does
/// before casting.
#[track_caller]
fn tap_all_mana(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
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

/// Taps every mana source `seat` has, except the ones printed `skip`.
///
/// [`tap_mana_except`] keeps one object; this keeps a whole printing, which
/// is how a test says "leave the Plains for the instant I am holding".
fn tap_all_mana_but(engine: &mut Engine<RegistryLookup>, seat: PlayerId, skip: Option<CardIndex>) {
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities {
        let printed = engine
            .state()
            .object(source)
            .and_then(|o| o.card)
            .map(|c| c.index);
        if skip.is_some() && printed == skip {
            continue;
        }
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
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
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            _ => break,
        }
    }
    asked
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

fn den_of_the_bugbear() -> CardIndex {
    card_index("f451b8f0-1ff5-4e8d-9f30-9352d83ed687")
}

fn hive_of_the_eye_tyrant() -> CardIndex {
    card_index("d17163d4-dd43-4de6-b7cf-576448160b7f")
}

fn thran_portal() -> CardIndex {
    card_index("926ce6a2-7bdd-4380-ac65-bc902ba0c284")
}
