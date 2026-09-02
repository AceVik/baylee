//! End-to-end: the client taps lands for a spell the engine has not offered.
//!
//! The engine reports a spell as castable only when the mana is already
//! floating, which is the correct rules answer and a hand that looks empty to
//! a player with three untapped Forests. `manaplan` decides which lands to
//! tap and `manasources` reads what they make; this is the test that the two
//! of them agree with the engine about all three — the plan, the taps, and
//! the spell that comes out the other end.
//!
//! Every action here goes through `LocalHost`, so a plan the engine would
//! refuse fails this test rather than leaving a player tapped out.

use baylee_client::host::{DuelHost, HostMessage, LocalHost};
use baylee_client::manasources;
use baylee_client_core::manaplan;
use baylee_core::ids::{CardIndex, PlayerId, PrintRef};
use baylee_core::preset::{
    AIProfile, DeckEntry, Finish, FormatId, GamePreset, HouseRules, PrintInfo, SeatController,
    SeatSpec,
};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::PlayerView;

/// Great Divide Guide — `{1}{G}`, a 1/2 with a printed mana ability of its
/// own, which is exactly the shape that makes this interesting: once it is on
/// the battlefield it is another source.
const SQUAD: &str = "79e69a91-d580-47fb-be76-1e32c50d2fa0";
/// Forest.
const FOREST: &str = "b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6";

fn card(oracle: &str) -> CardIndex {
    baylee_cards::by_oracle_id(oracle)
        .expect("the acceptance registry contains the card")
        .index
}

fn entry(oracle: &str) -> DeckEntry {
    DeckEntry {
        card: card(oracle),
        print: PrintRef::new(0),
    }
}

/// Seat 0 opens with three Forests on the table and a creature in hand.
fn preset() -> GamePreset {
    let deck: Vec<DeckEntry> = (0..60).map(|_| entry(FOREST)).collect();
    let seat = |ai: bool| SeatSpec {
        controller: if ai {
            SeatController::Ai(AIProfile::default())
        } else {
            SeatController::Open
        },
        capabilities: baylee_core::preset::SeatCapabilities::default(),
        deck: deck.clone(),
        sideboard: vec![],
        starting_life: None,
        starting_hand: None,
        starting_battlefield: vec![],
        emblems: vec![],
        team: None,
    };
    let mut preset = GamePreset {
        format: FormatId::Freeform,
        seed: 7,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: vec![seat(false), seat(true)],
    };
    preset.seats[0].starting_hand = Some(vec![entry(SQUAD), entry(FOREST)]);
    preset.seats[0].starting_battlefield = (0..3).map(|_| entry(FOREST)).collect();
    preset
}

/// Everything the test needs to see, which is what the client's own resource
/// holds: the last view and the last choice.
struct Table {
    host: LocalHost,
    view: Option<PlayerView>,
    pending: Option<Pending>,
}

impl Table {
    fn open() -> Self {
        let host = LocalHost::new(&preset(), PlayerId::new(0), &["You", "House AI"])
            .expect("the duel starts");
        let mut table = Self {
            host,
            view: None,
            pending: None,
        };
        table.drain();
        table
    }

    fn drain(&mut self) {
        for message in self.host.poll() {
            match message {
                HostMessage::View(view) => self.view = Some(*view),
                HostMessage::Choice(pending) => self.pending = Some(*pending),
                HostMessage::Failed(reason) => panic!("the engine refused: {reason}"),
                HostMessage::Static(_) => {}
            }
        }
    }

    fn submit(&mut self, action: PlayerAction) {
        self.host.submit(action);
        self.drain();
    }

    fn view(&self) -> &PlayerView {
        self.view.as_ref().expect("a view")
    }

    /// Answers everything that is not a main-phase priority, and stops there.
    ///
    /// The mulligan is kept and nothing is attacked; the point of the run is
    /// to arrive at the one moment where a player would be looking at their
    /// hand and their lands.
    fn walk_to_main(&mut self) {
        for _ in 0..200 {
            let Some(pending) = self.pending.clone() else {
                return;
            };
            match pending {
                Pending::Priority { player, .. }
                    if player == PlayerId::new(0)
                        && self.view().phase == baylee_view::Phase::FirstMain
                        && self.view().active == PlayerId::new(0) =>
                {
                    return;
                }
                Pending::Priority { player, .. } if player == PlayerId::new(0) => {
                    self.submit(PlayerAction::PassPriority);
                }
                Pending::Mulligan { player, .. } if player == PlayerId::new(0) => {
                    self.submit(PlayerAction::MulliganKeep);
                }
                Pending::ChooseAttackers { player, .. } if player == PlayerId::new(0) => {
                    self.submit(PlayerAction::DeclareAttackers { attackers: vec![] });
                }
                Pending::ChooseBlockers { player, .. } if player == PlayerId::new(0) => {
                    self.submit(PlayerAction::DeclareBlockers { blockers: vec![] });
                }
                _ => return,
            }
        }
        panic!("never reached a main phase");
    }

    fn legal(&self) -> &baylee_engine::choice::LegalActions {
        match self.pending.as_ref() {
            Some(Pending::Priority { legal, .. }) => legal,
            other => panic!("expected priority, got {other:?}"),
        }
    }
}

#[test]
fn the_client_taps_the_lands_a_spell_needs_and_then_casts_it() {
    let mut table = Table::open();
    table.walk_to_main();

    let spell = table
        .view()
        .hand
        .iter()
        .find(|c| c.name == "Great Divide Guide")
        .expect("the creature is in the opening hand")
        .id;

    // The premise: the engine has *not* offered it, because nothing is
    // floating. Without this line the rest of the test would pass for the
    // wrong reason.
    assert!(
        !table.legal().castable.contains(&spell),
        "the engine should not offer a spell whose mana is not floating"
    );

    // Three, not six. A Forest is offered twice by `LegalActions` — once as
    // the CR 305.6 shortcut and once as the `{T}: Add {G}` printed on the card
    // — and it can still only be tapped once.
    let sources = manasources::sources(table.view(), table.legal());
    assert_eq!(sources.len(), 3, "three untapped Forests");

    let cost = manasources::hand_cost(
        table
            .view()
            .hand
            .iter()
            .find(|c| c.id == spell)
            .expect("still in hand"),
    )
    .expect("a printed cost");
    let pool = table
        .view()
        .seat(PlayerId::new(0))
        .expect("own seat")
        .mana_pool;
    let plan = manaplan::plan(&cost, &pool, &sources).expect("{1}{G} out of three Forests");
    assert_eq!(plan.taps(), 2, "two Forests, not three");

    // Exactly what the client's own run does: tap, then cast, re-checking the
    // engine's offer at every step.
    for step in &plan.steps {
        assert!(
            table.legal().mana_abilities.contains(&step.source),
            "the engine still offers the land the plan picked"
        );
        assert_eq!(step.color, None, "a Forest is never asked which colour");
        table.submit(PlayerAction::ActivateManaAbility {
            source: step.source,
        });
    }

    assert!(
        table.legal().castable.contains(&spell),
        "with the mana floating the engine offers the spell"
    );
    table.submit(PlayerAction::CastSpell { card: spell });

    // Cast, resolved through both seats passing, and on the battlefield.
    for _ in 0..40 {
        if table
            .view()
            .battlefield
            .iter()
            .any(|o| o.name == "Great Divide Guide")
        {
            return;
        }
        match table.pending.clone() {
            Some(Pending::Priority { player, .. }) if player == PlayerId::new(0) => {
                table.submit(PlayerAction::PassPriority);
            }
            _ => break,
        }
    }
    panic!("the spell never reached the battlefield");
}

/// The other half of the claim: a spell nothing on the table can pay for is
/// not offered a plan either. A client that says yes here would tap two lands
/// and then stop, which is worse than saying no.
#[test]
fn a_spell_the_lands_cannot_pay_for_gets_no_plan() {
    let mut table = Table::open();
    table.walk_to_main();

    let sources = manasources::sources(table.view(), table.legal());
    let pool = table
        .view()
        .seat(PlayerId::new(0))
        .expect("own seat")
        .mana_pool;

    // Three Forests, and a cost with a blue pip in it.
    let cost = baylee_core::mana::ManaCost::try_parse("{1}{U}").expect("a valid cost");
    assert!(manaplan::plan(&cost, &pool, &sources).is_none());

    // …and one that is simply too expensive.
    let cost = baylee_core::mana::ManaCost::try_parse("{7}").expect("a valid cost");
    assert!(manaplan::plan(&cost, &pool, &sources).is_none());
}

/// The manual half, which is the half that did not exist at all: a client
/// that cannot activate an ability cannot play a game, whatever it does with
/// mana automatically.
#[test]
fn a_permanent_offers_exactly_what_it_can_do_and_never_the_same_tap_twice() {
    use baylee_client::abilities;
    use baylee_client_core::interaction::Interaction;

    let mut table = Table::open();
    table.walk_to_main();

    let interaction = Interaction::new(table.pending.clone().expect("priority"), PlayerId::new(0));
    let forest = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name == "Forest")
        .expect("a Forest on the table")
        .id;

    // One button, not two. A Forest is offered by the engine as the CR 305.6
    // shortcut *and* as the `{T}: Add {G}` printed on the card, and it can
    // still only be tapped once.
    let options = abilities::options(table.view(), &interaction, forest);
    assert_eq!(options.len(), 1, "{options:?}");
    assert_eq!(options[0].label, "Tap for G");

    // And it is an action the engine takes.
    table.submit(options[0].action.clone());
    let pool = table
        .view()
        .seat(PlayerId::new(0))
        .expect("own seat")
        .mana_pool;
    assert_eq!(pool.green, 1, "the Forest made a green mana");

    // A card in hand is not a permanent and offers nothing to activate.
    let in_hand = table.view().hand.first().expect("a hand").id;
    assert!(abilities::options(table.view(), &interaction, in_hand).is_empty());
}
