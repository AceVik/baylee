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
/// cycle on a land early and the other off a land early. Twenty-one cards
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
    "Deserted Beach",
    "Dreamroot Cascade",
    "Haunted Ridge",
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
