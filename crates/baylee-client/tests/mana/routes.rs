//! What the client offers to tap lands for, on tables a real engine dealt: a
//! card cast from somewhere other than the hand, or as another face, at a
//! price of its own (flashback, CR 702.34a; foretell, CR 702.143a; disturb,
//! CR 702.146a; an Adventure, CR 716.3), an ability activated from a
//! graveyard, and mana that may be spent only on some spells (CR 106.6),
//! which is never offered for a spell it may not pay for.
//!
//! Each table is played to the moment and then read the way the frame loop
//! reads it: the view the engine projects for the seat, the question it
//! asks, and `rebuild_board`. A card the engine would cast once its mana is
//! made, and which the seat's mana can pay for, is lit (`Duel::reachable`);
//! one it cannot pay for is not, and a lit one taken through the click path
//! ends on the stack.

use baylee_client::Duel;
use baylee_client_core::interaction::Interaction;
use baylee_core::ids::{ObjectId, PlayerId, PrintRef};
use baylee_core::mana::ManaColor;
use baylee_core::preset::{DeckEntry, GamePreset, SeatController};
use baylee_engine::choice::{Pending, PlayerAction, timeout_answer};
use baylee_engine::engine::Engine;
use baylee_engine::turn::Step;
use baylee_gamehost::{RegistryLookup, SeatContext, owed_payment, player_view};
use baylee_view::PlayerView;

const ME: PlayerId = PlayerId::new(0);

/// An answer to a question the table is asked on the way, `None` for its
/// timeout answer.
type Answer<'a> = &'a mut dyn FnMut(&PlayerView, &Pending) -> Option<PlayerAction>;

fn named(name: &str) -> DeckEntry {
    DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: baylee_cards::decks::by_name(name).unwrap_or_else(|| panic!("{name} is in the pool")),
        print: PrintRef::new(0),
    }
}

/// Seat 0 starts, holding `hand` with `mine` on the table; seat 1 has
/// `theirs`. Both libraries are `library`.
fn preset(library: &str, hand: &[&str], mine: &[&str], theirs: &[&str]) -> GamePreset {
    let land = named(library);
    let mut preset =
        baylee_cards::decks::probe_preset(41, land.card).expect("a probe table for the land");
    preset.house_rules.starting_player = Some(0);
    for seat in &mut preset.seats {
        seat.controller = SeatController::Open;
        seat.deck = (0..40).map(|_| land).collect();
        seat.starting_hand = Some(vec![]);
        seat.starting_battlefield.clear();
    }
    preset.seats[0].starting_hand = Some(hand.iter().copied().map(named).collect());
    preset.seats[0].starting_battlefield = mine.iter().copied().map(named).collect();
    preset.seats[1].starting_battlefield = theirs.iter().copied().map(named).collect();
    preset
}

/// A game and the client resource that reads it.
struct Table {
    engine: Engine<RegistryLookup>,
    duel: Duel,
}

impl Table {
    /// The game at seat 0's first main phase.
    fn open(library: &str, hand: &[&str], mine: &[&str], theirs: &[&str]) -> Self {
        let engine = Engine::new(&preset(library, hand, mine, theirs), RegistryLookup)
            .expect("the duel starts");
        let mut table = Self {
            engine,
            duel: Duel::default(),
        };
        table.play_to_my_main(1);
        table
    }

    /// Answers everything with its timeout answer until seat 0 has priority in
    /// its own precombat main phase of turn `turn` or later, the stack empty.
    fn play_to_my_main(&mut self, turn: u32) {
        for _ in 0..400 {
            let pending = self.engine.pending().clone();
            let state = self.engine.state();
            if matches!(pending, Pending::Priority { player, .. } if player == ME)
                && state.turn.active == ME
                && state.turn.step == Step::Main
                && state.turn.number >= turn
                && state.zones.stack_is_empty()
            {
                self.sync();
                return;
            }
            let player = pending.asked().expect("somebody is asked");
            let answer = timeout_answer(&pending).expect("a default answer");
            self.engine.apply(player, answer).expect("a legal default");
        }
        panic!("never reached seat 0's main phase");
    }

    /// The client's view of the game, refreshed as a frame refreshes it.
    fn sync(&mut self) {
        let pending = self.engine.pending().clone();
        let seq = self.duel.view.as_ref().map_or(0, |v| v.seq + 1);
        let view = player_view(
            self.engine.state(),
            ME,
            seq,
            Some(&pending),
            &SeatContext {
                awaiting: baylee_gamehost::view::awaiting_for(&self.engine, ME),
                controls: &baylee_gamehost::view::player_controls(&self.engine),
                decision_player: pending.asked(),
                controlled_players: self.engine.controlled_players(ME),
                deciding: baylee_gamehost::view::deciding(&self.engine),
                owed: owed_payment(&self.engine),
                ..SeatContext::default()
            },
            &[],
        );
        self.duel.view = Some(view);
        self.duel.interaction = Some(Interaction::new(pending, ME));
        baylee_client::rebuild_board(&mut self.duel);
    }

    fn view(&self) -> &PlayerView {
        self.duel.view.as_ref().expect("synced")
    }

    fn apply(&mut self, action: PlayerAction) {
        let player = self.engine.pending().asked().expect("somebody is asked");
        let shown = format!("{action:?}");
        self.engine
            .apply(player, action)
            .unwrap_or_else(|e| panic!("{shown} refused: {e:?}"));
        self.sync();
    }

    /// Answers until the stack is empty and seat 0 has priority: priority is
    /// passed, every other question answered by `answer` or its timeout
    /// answer.
    fn resolve(&mut self, answer: Answer<'_>) {
        for _ in 0..200 {
            if self.engine.state().zones.stack_is_empty()
                && matches!(self.engine.pending(), Pending::Priority { player, .. } if *player == ME)
            {
                return;
            }
            let pending = self.engine.pending().clone();
            let action = answer(self.view(), &pending)
                .or_else(|| fallback(self.view(), &pending))
                .unwrap_or_else(|| panic!("no answer to {pending:?}"));
            self.apply(action);
        }
        panic!("the stack never emptied");
    }

    /// Seat 0's permanents called `name`.
    fn mine(&self, name: &str) -> Vec<ObjectId> {
        self.view()
            .battlefield
            .iter()
            .filter(|o| o.controller == ME && o.name.as_ref() == name)
            .map(|o| o.id)
            .collect()
    }

    /// Seat 0's card called `name` wherever its view shows it: the hand, its
    /// graveyard, an exile zone.
    fn card(&self, name: &str) -> ObjectId {
        let view = self.view();
        view.hand
            .iter()
            .find(|c| c.name == name)
            .map(|c| c.id)
            .or_else(|| {
                view.graveyards
                    .iter()
                    .chain(&view.exile)
                    .flatten()
                    .find(|o| o.owner == ME && o.name.as_ref() == name)
                    .map(|o| o.id)
            })
            .unwrap_or_else(|| panic!("{name} is in no zone seat 0 sees"))
    }

    /// Takes a lit card through the click path: the chooser's first way if
    /// one opens, arm, fire, the run's taps and the cast, each question on
    /// the way answered by `answer` or its timeout answer, until a spell is
    /// on the stack.
    fn click_and_cast(&mut self, card: ObjectId, answer: Answer<'_>) {
        let before = self.stack_len();
        baylee_client::input::activate_card(&mut self.duel, card);
        if self.duel.cast_menu.is_some() {
            baylee_client::input::pick_choice(&mut self.duel, 0);
        }
        if self.duel.armed.is_some() {
            baylee_client::input::fire_armed(&mut self.duel);
        }
        for _ in 0..40 {
            if self.stack_len() > before {
                return;
            }
            baylee_client::advance_mana_run(&mut self.duel);
            baylee_client::take_the_chosen_cast_mode(&mut self.duel);
            let sent = self.duel.take_outbox();
            if sent.is_empty() {
                assert_eq!(
                    self.duel.last_error,
                    None,
                    "the run stopped: {:?}",
                    self.engine.pending()
                );
                let pending = self.engine.pending().clone();
                let action = answer(self.view(), &pending)
                    .or_else(|| match pending {
                        Pending::Priority { .. } => None,
                        _ => fallback(self.view(), &pending),
                    })
                    .unwrap_or_else(|| panic!("nothing sent, nothing to answer: {pending:?}"));
                self.apply(action);
            }
            for action in sent {
                self.apply(action);
            }
        }
        panic!("the spell never reached the stack");
    }

    fn stack_len(&self) -> usize {
        self.engine
            .state()
            .zones
            .list(baylee_engine::zone::ZoneLocation::Stack)
            .len()
    }

    fn lit(&self, card: ObjectId) -> bool {
        self.duel.reachable.contains(&card)
    }
}

fn nothing(_: &PlayerView, _: &Pending) -> Option<PlayerAction> {
    None
}

/// What a player who does not care answers: their timeout answer, themselves
/// for a player, and the first cards of their hand for a discard.
fn fallback(view: &PlayerView, pending: &Pending) -> Option<PlayerAction> {
    match pending {
        Pending::ChoosePlayer { options, .. } => options
            .iter()
            .find(|p| **p == ME)
            .or(options.first())
            .map(|p| PlayerAction::ChoosePlayer(*p)),
        Pending::ChooseCards { options, min, .. } => Some(PlayerAction::ChooseObjects {
            objects: options.iter().copied().take(usize::from(*min)).collect(),
        }),
        Pending::ChooseTargets {
            options,
            player_options,
            min,
            ..
        } => Some(PlayerAction::ChooseTargets {
            objects: options.iter().copied().take(*min as usize).collect(),
            players: player_options
                .iter()
                .copied()
                .take((*min as usize).saturating_sub(options.len()))
                .collect(),
        }),
        Pending::DiscardChoice { count, .. } => Some(PlayerAction::ChooseObjects {
            objects: view
                .hand
                .iter()
                .map(|c| c.id)
                .take(usize::from(*count))
                .collect(),
        }),
        _ => timeout_answer(pending),
    }
}

/// Taps one of seat 0's `name` permanents for the mana ability at `index`,
/// naming `color` if it asks.
fn tap(table: &mut Table, name: &str, index: u32, color: ManaColor) {
    let source = table.mine(name)[0];
    table.apply(PlayerAction::ActivateAbility {
        source,
        ability_index: index,
    });
    if matches!(table.engine.pending(), Pending::ChooseColor { .. }) {
        table.apply(PlayerAction::ChooseColor(color));
    }
}

// ------------------------------------------------------------ restricted mana
//
// Mana spent only on some spells (CR 106.6) is never offered for a spell it
// may not pay for. What it may pay for is the engine's to say, and where the
// planners read it off the view they withhold rather than guess.

/// Ancient Ziggurat: "{T}: Add one mana of any color. Spend this mana only to
/// cast a creature spell." Two of them never light Lightning Bolt.
#[test]
fn ziggurat_mana_never_pays_for_an_instant() {
    let table = Table::open(
        "Mountain",
        &["Lightning Bolt"],
        &["Ancient Ziggurat", "Ancient Ziggurat"],
        &[],
    );
    let bolt = table.card("Lightning Bolt");
    assert!(!table.lit(bolt), "Ziggurat mana never pays for an instant");
}

/// Pillar of the Paruns: "Spend this mana only to cast a multicolored
/// spell." Two never light Grizzly Bears.
#[test]
fn pillar_mana_never_pays_for_a_mono_coloured_spell() {
    let table = Table::open(
        "Forest",
        &["Grizzly Bears"],
        &["Pillar of the Paruns", "Pillar of the Paruns"],
        &[],
    );
    let bears = table.card("Grizzly Bears");
    assert!(
        !table.lit(bears),
        "Pillar mana never pays for a green spell"
    );
}

/// Eldrazi Temple: "{T}: Add {C}{C}. Spend this mana only to cast colorless
/// Eldrazi spells or activate abilities of colorless Eldrazi." Beside a
/// Forest, two make three mana for a green Eldrazi, Kozilek's Predator's
/// {3}{G} out of reach.
#[test]
fn temple_mana_never_pays_for_a_coloured_eldrazi() {
    let table = Table::open(
        "Forest",
        &["Kozilek's Predator"],
        &["Eldrazi Temple", "Eldrazi Temple", "Forest"],
        &[],
    );
    let predator = table.card("Kozilek's Predator");
    assert!(
        !table.lit(predator),
        "the Temples' {{C}}{{C}} pays a colourless Eldrazi only"
    );
}

/// Cavern of Souls naming Elf, its green floating: a Mountain and that green
/// never pay a Bear's {1}{G}.
#[test]
fn floating_cavern_mana_never_pays_for_another_type() {
    let mut table = Table::open(
        "Mountain",
        &["Cavern of Souls", "Grizzly Bears", "Elvish Archdruid"],
        &["Mountain"],
        &[],
    );
    let cavern = table.card("Cavern of Souls");
    table.apply(PlayerAction::PlayLand { card: cavern });
    table.apply(PlayerAction::ChooseSubtype(
        baylee_core::generated::subtypes::creature::ELF,
    ));
    tap(&mut table, "Cavern of Souls", 1, ManaColor::Green);
    let bears = table.card("Grizzly Bears");
    assert!(
        !table.lit(bears),
        "green that pays only for an Elf and a Mountain do not pay {{1}}{{G}} for a Bear"
    );
}

// ---------------------------------------------------------- other zones

/// Faithless Looting cast, then flashed back from the graveyard for {2}{R}
/// with three Mountains.
#[test]
fn faithless_looting_is_flashed_back() {
    let mut table = Table::open(
        "Mountain",
        &["Faithless Looting", "Mountain", "Mountain"],
        &["Mountain", "Mountain", "Mountain", "Mountain"],
        &[],
    );
    let looting = table.card("Faithless Looting");
    table.click_and_cast(looting, &mut nothing);
    table.resolve(&mut nothing);
    let looting = table.card("Faithless Looting");
    assert!(
        table.lit(looting),
        "three Mountains pay the flashback {{2}}{{R}}"
    );
    table.click_and_cast(looting, &mut nothing);
}

/// Deep Analysis flashed back for {1}{U} and 3 life.
#[test]
fn deep_analysis_is_flashed_back_for_mana_and_life() {
    let mut table = Table::open(
        "Island",
        &["Deep Analysis"],
        &["Island", "Island", "Island", "Island", "Island", "Island"],
        &[],
    );
    let analysis = table.card("Deep Analysis");
    table.click_and_cast(analysis, &mut nothing);
    table.resolve(&mut nothing);
    let analysis = table.card("Deep Analysis");
    assert!(
        table.lit(analysis),
        "two Islands and 3 life pay the flashback"
    );
    table.click_and_cast(analysis, &mut nothing);
}

/// Doomskar foretold on turn 1 and cast from exile on turn 3 for its
/// foretell cost {1}{W}{W} with three Plains: its printed {3}{W}{W} is
/// beyond them.
#[test]
fn a_foretold_card_is_cast_for_its_foretell_cost() {
    let mut table = Table::open(
        "Plains",
        &["Doomskar"],
        &["Plains", "Plains", "Plains"],
        &[],
    );
    let doomskar = table.card("Doomskar");
    for plains in table.mine("Plains").into_iter().take(2) {
        table.apply(PlayerAction::ActivateManaAbility { source: plains });
    }
    table.apply(PlayerAction::Foretell { card: doomskar });
    table.play_to_my_main(3);
    let foretold = table.card("Doomskar");
    assert!(
        table.lit(foretold),
        "three Plains pay the foretell cost {{1}}{{W}}{{W}}"
    );
    table.click_and_cast(foretold, &mut nothing);
}

/// Brazen Borrower with two Islands: the creature's {1}{U}{U} is out of
/// reach, Petty Theft's {1}{U} is not.
#[test]
fn an_adventure_is_reached_for_from_the_hand() {
    let mut table = Table::open(
        "Island",
        &["Brazen Borrower"],
        &["Island", "Island"],
        &["Grizzly Bears"],
    );
    let borrower = table.card("Brazen Borrower");
    assert!(
        table.lit(borrower),
        "two Islands pay Petty Theft's {{1}}{{U}}"
    );
    table.click_and_cast(borrower, &mut nothing);
}

/// Malevolent Hermit discarded to Faithless Looting, then disturbed from the
/// graveyard as Benevolent Geist for {2}{U}. With two Islands it is out of
/// reach (the front's {1}{U} is not its price); with three it is lit.
#[test]
fn a_disturb_card_is_priced_at_its_disturb_cost() {
    for (islands, lit) in [(2, false), (3, true)] {
        let mut mine = vec!["Mountain"];
        mine.extend(std::iter::repeat_n("Island", islands));
        let mut table = Table::open(
            "Mountain",
            &["Faithless Looting", "Malevolent Hermit"],
            &mine,
            &[],
        );
        let looting = table.card("Faithless Looting");
        let hermit = table.card("Malevolent Hermit");
        let mountain = table.mine("Mountain")[0];
        table.apply(PlayerAction::ActivateManaAbility { source: mountain });
        table.apply(PlayerAction::CastSpell { card: looting });
        table.resolve(&mut |_, pending| match pending {
            Pending::ChooseCards { options, min, .. } if options.contains(&hermit) => {
                Some(PlayerAction::ChooseObjects {
                    objects: std::iter::once(hermit)
                        .chain(options.iter().copied().filter(|id| *id != hermit))
                        .take(usize::from(*min))
                        .collect(),
                })
            }
            _ => None,
        });
        let geist = table.card("Malevolent Hermit");
        assert_eq!(
            table.lit(geist),
            lit,
            "with {islands} Islands the disturb cost {{2}}{{U}} is {}",
            if lit { "paid" } else { "out of reach" }
        );
        if lit {
            table.click_and_cast(geist, &mut nothing);
        }
    }
}

/// Cavern of Souls naming Elf, nothing floating: the click casts the Elf and
/// its payment window is paid with the Cavern's green.
#[test]
fn a_graveyard_activation_is_reached_for() {
    let mut table = Table::open(
        "Mountain",
        &["Faithless Looting", "Drownyard Temple"],
        &["Mountain", "Mountain", "Mountain", "Mountain"],
        &[],
    );
    let looting = table.card("Faithless Looting");
    let temple = table.card("Drownyard Temple");
    let mountain = table.mine("Mountain")[0];
    table.apply(PlayerAction::ActivateManaAbility { source: mountain });
    table.apply(PlayerAction::CastSpell { card: looting });
    table.resolve(&mut |_, pending| match pending {
        Pending::ChooseCards { options, min, .. } if options.contains(&temple) => {
            Some(PlayerAction::ChooseObjects {
                objects: std::iter::once(temple)
                    .chain(options.iter().copied().filter(|id| *id != temple))
                    .take(usize::from(*min))
                    .collect(),
            })
        }
        _ => None,
    });
    let temple = table.card("Drownyard Temple");
    assert_eq!(
        table.duel.reach_of(temple),
        Some(baylee_client::Reach::Taps),
        "three Mountains pay the Temple's {{3}}"
    );
}

// ------------------------------------------------------------ hybrid costs

/// Temur Tawnyback, `{2/G}{2/U}{2/R}`, beside an Island, Obelisk of Bant and
/// three Plains (the board of the LLM-seat report of 11.11.2026): the
/// Island's blue, the Obelisk's green, and two Plains for `{2/R}` pay it
/// (CR 107.4e, each symbol on its own). Read all coloured or all generic it
/// was paid by neither, and never lit.
#[test]
fn a_cost_of_twobrids_is_paid_by_a_mix() {
    let mut table = Table::open(
        "Plains",
        &["Temur Tawnyback"],
        &["Island", "Obelisk of Bant", "Plains", "Plains", "Plains"],
        &[],
    );
    let beast = table.card("Temur Tawnyback");
    assert!(table.lit(beast), "five sources pay {{2/G}}{{2/U}}{{2/R}}");
    table.click_and_cast(beast, &mut nothing);
}

// ------------------------------------- restricted mana, what it pays for
//
// Fault 69: the engine states what restricted mana may pay for
// (`LegalActions::restricted_taps`, `restricted_floating`), and every
// restricted tap and every restricted mana floating it names is planned like
// any other mana (`baylee_ai::spend`), in the reach and in the window.

/// Ancient Ziggurat: "{T}: Add one mana of any color. Spend this mana only to
/// cast a creature spell." Two of them pay Grizzly Bears' {1}{G}, and nothing
/// else on the table does; they never pay for Lightning Bolt.
#[test]
fn two_ziggurats_pay_for_a_creature_and_not_for_a_bolt() {
    let mut table = Table::open(
        "Mountain",
        &["Grizzly Bears", "Lightning Bolt"],
        &["Ancient Ziggurat", "Ancient Ziggurat"],
        &[],
    );
    let bears = table.card("Grizzly Bears");
    let bolt = table.card("Lightning Bolt");
    assert!(
        table.lit(bears),
        "two Ziggurats pay {{1}}{{G}} for a creature"
    );
    assert!(!table.lit(bolt), "Ziggurat mana never pays for an instant");
    table.click_and_cast(bears, &mut nothing);
}

/// The Ziggurat's red already floating and a Forest untapped: the creature
/// takes the Forest's one tap.
#[test]
fn ziggurat_mana_floating_and_a_forest_pay_for_a_creature() {
    let mut table = Table::open(
        "Forest",
        &["Grizzly Bears"],
        &["Ancient Ziggurat", "Forest"],
        &[],
    );
    tap(&mut table, "Ancient Ziggurat", 0, ManaColor::Red);
    let bears = table.card("Grizzly Bears");
    assert!(
        table.lit(bears),
        "red Ziggurat mana floating and a Forest pay {{1}}{{G}}"
    );
    assert_eq!(
        baylee_client::mana_for(&table.duel, bears).map(|plan| plan.taps()),
        Some(1),
        "one tap: the Forest"
    );
    table.click_and_cast(bears, &mut nothing);
}

/// Pillar of the Paruns: "Spend this mana only to cast a multicolored
/// spell." Two pay Watchwolf's {G}{W}, never Grizzly Bears'.
#[test]
fn two_pillars_pay_for_a_gold_spell_and_not_for_a_green_one() {
    let mut table = Table::open(
        "Forest",
        &["Watchwolf", "Grizzly Bears"],
        &["Pillar of the Paruns", "Pillar of the Paruns"],
        &[],
    );
    let wolf = table.card("Watchwolf");
    let bears = table.card("Grizzly Bears");
    assert!(
        table.lit(wolf),
        "two Pillars pay {{G}}{{W}} for a gold spell"
    );
    assert!(
        !table.lit(bears),
        "Pillar mana never pays for a green spell"
    );
    table.click_and_cast(wolf, &mut nothing);
}

/// Eldrazi Temple: "{T}: Add {C}{C}. Spend this mana only to cast colorless
/// Eldrazi spells or activate abilities of colorless Eldrazi." Two Temples
/// and four Wastes are Eldrazi Devastator's {8}.
#[test]
fn two_temples_pay_for_a_colourless_eldrazi() {
    let mut table = Table::open(
        "Wastes",
        &["Eldrazi Devastator"],
        &[
            "Eldrazi Temple",
            "Eldrazi Temple",
            "Wastes",
            "Wastes",
            "Wastes",
            "Wastes",
        ],
        &[],
    );
    let devastator = table.card("Eldrazi Devastator");
    assert!(table.lit(devastator), "2 + 2 + 4 Wastes pay {{8}}");
    table.click_and_cast(devastator, &mut nothing);
}

/// The Temples' {C}{C} is never spent on a green Eldrazi: beside a Forest
/// they make three mana for Kozilek's Predator's {3}{G}.
#[test]
fn cavern_mana_pays_for_the_named_type_only() {
    let mut table = Table::open(
        "Mountain",
        &["Cavern of Souls", "Elvish Archdruid", "Grizzly Bears"],
        &["Forest", "Mountain"],
        &[],
    );
    let cavern = table.card("Cavern of Souls");
    table.apply(PlayerAction::PlayLand { card: cavern });
    let elf = baylee_core::generated::subtypes::creature::ELF;
    let Pending::ChooseSubtype { options, .. } = table.engine.pending().clone() else {
        panic!("Cavern of Souls asks for a type");
    };
    assert!(options.contains(&elf));
    table.apply(PlayerAction::ChooseSubtype(elf));
    let archdruid = table.card("Elvish Archdruid");
    let bears = table.card("Grizzly Bears");
    assert!(
        table.lit(archdruid),
        "Cavern's green, a Forest and a Mountain pay {{1}}{{G}}{{G}} for an Elf"
    );
    assert!(
        table.lit(bears),
        "the Forest's green and the Cavern's {{C}} pay {{1}}{{G}} for a Bear"
    );
    tap(&mut table, "Cavern of Souls", 1, ManaColor::Green);
    assert!(
        table.lit(archdruid),
        "Cavern's green floating, a Forest and a Mountain still pay for the Elf"
    );
    table.click_and_cast(archdruid, &mut nothing);
}

/// The Cavern's green floating is no Bear's: the Mountain and the Forest
/// would pay {1}{G} only with the Forest's green, which the Elf took.
#[test]
fn a_click_pays_an_elf_with_cavern_green() {
    let mut table = Table::open(
        "Mountain",
        &["Cavern of Souls", "Elvish Archdruid"],
        &["Forest", "Mountain"],
        &[],
    );
    let cavern = table.card("Cavern of Souls");
    table.apply(PlayerAction::PlayLand { card: cavern });
    table.apply(PlayerAction::ChooseSubtype(
        baylee_core::generated::subtypes::creature::ELF,
    ));
    let archdruid = table.card("Elvish Archdruid");
    assert!(table.lit(archdruid));
    table.click_and_cast(archdruid, &mut nothing);
}

/// Dismember, `{1}{B/P}{B/P}`, at 20 life beside a Mountain: lit, and the
/// click casts it with the Mountain and 4 life (CR 107.4f).
#[test]
fn phyrexian_mana_is_paid_with_life() {
    let mut table = Table::open(
        "Mountain",
        &["Dismember"],
        &["Mountain"],
        &["Grizzly Bears"],
    );
    let dismember = table.card("Dismember");
    assert!(table.lit(dismember), "a Mountain and 4 life pay it");
    table.click_and_cast(dismember, &mut |_, pending| match pending {
        Pending::YesNo { .. } => Some(PlayerAction::YesNo(true)),
        _ => None,
    });
}
