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
//!
//! `main.rs` and not `mod.rs`, which is the one place in this workspace that
//! departs from the form the rest of the splits took: Cargo discovers an
//! integration test as `tests/<name>.rs` or `tests/<name>/main.rs` and as
//! nothing else, so a `mod.rs` here would drop the whole target without a
//! word — the 25 tests below would simply stop running and the suite would
//! stay green.

mod arming;
mod cast_modes;
mod chooser;
mod pips;
mod planning;
mod sheet;

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
        commanders: vec![],
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
        Self::open_with(&preset())
    }

    fn open_with(preset: &GamePreset) -> Self {
        let host = LocalHost::new(preset, PlayerId::new(0), &["You", "House AI"])
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
                HostMessage::Static(_) | HostMessage::Curtain => {}
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

/// Bloodstained Mire — `{T}, Sacrifice this, Pay 1 life:` a fetch. Chosen
/// because it is the shape a mana ability is not: it uses the stack, it costs
/// more than a tap, and the client has to name it without help from a colour.
const MIRE: &str = "fc0707c7-d504-4ccf-a0d2-3eb6e26e7a57";

/// The same seat, with a fetchland already on the table.
fn preset_with_a_fetchland() -> GamePreset {
    let mut preset = preset();
    preset.seats[0].starting_battlefield.push(entry(MIRE));
    preset
}

/// Yavimaya Coast — a painland, and therefore a permanent with two mana
/// abilities: `{T}: Add {C}` and `{T}: Add {G} or {U}` for a life.
const PAINLAND: &str = "40b36bc6-c185-4bda-99e7-0118953c2c97";

fn preset_with_a_painland() -> GamePreset {
    let mut preset = preset();
    preset.seats[0].starting_battlefield.push(entry(PAINLAND));
    preset
}

/// Karn, the Great Creator — a planeswalker, and therefore a permanent with
/// two abilities *neither* of which makes mana.
const WALKER: &str = "a20dd48d-d344-4db1-b0e9-a2b71c3cc9d1";

fn preset_with_a_planeswalker() -> GamePreset {
    let mut preset = preset();
    preset.seats[0].starting_battlefield.push(entry(WALKER));
    preset
}

/// Chromatic Lantern — "Lands you control have `{T}: Add one mana of any
/// color`". The card that turns a fetchland into a mana source in the eyes of
/// `LegalActions`, and therefore the card that broke the click path.
const LANTERN: &str = "539f5396-d99a-417d-a84c-dff7930b5900";

/// Sea Gate Loremaster — `{T}: Draw a card for each Ally you control`. The
/// shape the owner named: a cost paid entirely by tapping the permanent, and
/// nothing else.
const LOREMASTER: &str = "6eed122b-9760-47fd-8ba2-adeda8054e0d";

/// Ancestral Vision — no mana cost at all, and Suspend 4—`{U}`.
const VISION: &str = "9728dec9-d482-4c7a-8cdc-44d010dc878d";

/// Island.
const ISLAND: &str = "b2c6aa39-2d2a-459c-a555-fb48ba993373";

/// Seat 0 opens with Ancestral Vision in hand and one untapped Island.
fn suspend_preset() -> GamePreset {
    let deck: Vec<DeckEntry> = (0..60).map(|_| entry(ISLAND)).collect();
    let seat = |ai: bool| SeatSpec {
        controller: if ai {
            SeatController::Ai(AIProfile::default())
        } else {
            SeatController::Open
        },
        capabilities: baylee_core::preset::SeatCapabilities::default(),
        deck: deck.clone(),
        sideboard: vec![],
        commanders: vec![],
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
    preset.seats[0].starting_hand = Some(vec![entry(VISION)]);
    preset.seats[0].starting_battlefield = vec![entry(ISLAND)];
    preset
}

/// Tundra. Two basic land types, so the engine offers no CR 305.6 shortcut
/// for it at all: its mana is the printed `{T}: Add {W} or {U}`, and that
/// ability asks.
const TUNDRA: &str = "02418479-9455-417f-a6a1-004356faff37";

/// Harabaz Druid — `{T}: Add X mana of any one color, where X is the number
/// of Allies you control`. It is an Ally itself, so X is at least one, and it
/// is the card whose amount no planner can read (`Amount::CountOf`).
const HARABAZ: &str = "ead985ec-f29f-4a3b-b8b1-061142cc5bd1";

/// Homeward Path — `{T}: Add {C}` **and** `{T}: Each player gains control of
/// all creatures they own`. Two free taps, both legal at once and one of them
/// mana: the card that makes a sheet with a pip on it rather than a bubble.
const HOMEWARD: &str = "cb8ec2e4-8223-4172-8f2c-37c918a573fa";

/// Seat 0 opens with a Tundra and a Harabaz Druid on the table.
///
/// Both are the owner's fourth point in one board: a land that asks which of
/// two colours, and a creature whose only ability is mana and whose amount is
/// a count of the battlefield.
fn bubble_preset() -> GamePreset {
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
        commanders: vec![],
        starting_life: None,
        starting_hand: None,
        starting_battlefield: vec![],
        emblems: vec![],
        team: None,
    };
    let mut preset = GamePreset {
        format: FormatId::Freeform,
        seed: 11,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: vec![seat(false), seat(true)],
    };
    preset.seats[0].starting_battlefield = vec![entry(TUNDRA), entry(HARABAZ)];
    preset
}

// --------------------------------------------------------------------- AZ
//
// The mana planner used to pick the *way* a spell was cast, silently, by
// floating exactly the printed cost and then asking the engine to cast it.
// The engine counts a spell's ways against the pool as it stands (it has no
// planner and cannot do otherwise), so by then there was only ever one way
// left and nothing to ask about. The repair is that the client asks first —
// `Duel::cast_menu` — and remembers the answer until the engine's own
// question arrives, several round trips later.
//
// Both tests below go through the click and the row press, never through a
// hand-built `PlayerAction`: what is claimed is about buttons.

/// Reveillark — `{4}{W}`, and Evoke `{5}{W}`. The alternative cost is the
/// **dearer** one, which is the half a planner can never reach on its own.
const REVEILLARK: &str = "1be13ede-98f8-497e-800c-03e5802932b3";

/// Solitude — `{3}{W}{W}`, or exile a white card from your hand. The
/// alternative is **free**, so the engine offers the card as castable off an
/// empty pool and the printed cost is the one that was unreachable.
const SOLITUDE: &str = "dcb9c2a7-ae54-4ddc-a567-640bf4bf4366";

/// Plains.
const PLAINS: &str = "bc71ebf6-2056-41f7-be35-b2e5c34afa99";

/// Seat 0 with `lands` untapped Plains on the table and `hand` in hand.
fn white_preset(hand: &[&str], lands: usize) -> GamePreset {
    let deck: Vec<DeckEntry> = (0..60).map(|_| entry(PLAINS)).collect();
    let seat = |ai: bool| SeatSpec {
        controller: if ai {
            SeatController::Ai(AIProfile::default())
        } else {
            SeatController::Open
        },
        capabilities: baylee_core::preset::SeatCapabilities::default(),
        deck: deck.clone(),
        sideboard: vec![],
        commanders: vec![],
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
    preset.seats[0].starting_hand = Some(hand.iter().copied().map(entry).collect());
    preset.seats[0].starting_battlefield = (0..lands).map(|_| entry(PLAINS)).collect();
    preset
}

/// The client resource, refreshed the way the frame loop refreshes it.
fn refresh(duel: &mut baylee_client::Duel, table: &Table) {
    use baylee_client_core::interaction::Interaction;
    duel.view = Some(table.view().clone());
    duel.interaction = Some(Interaction::new(
        table.pending.clone().expect("a question"),
        PlayerId::new(0),
    ));
    baylee_client::rebuild_board(duel);
}

fn id_in_hand(table: &Table, name: &str) -> baylee_core::ids::ObjectId {
    table
        .view()
        .hand
        .iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("{name} is in the opening hand"))
        .id
}

fn untapped_lands(table: &Table) -> usize {
    table
        .view()
        .battlefield
        .iter()
        .filter(|o| {
            o.controller == PlayerId::new(0)
                && o.types.contains(baylee_core::types::TypeSet::LAND)
                && !o.status.contains(baylee_view::ObjectStatus::TAPPED)
        })
        .count()
}

/// Runs the two systems the frame loop runs after a deed is fired, one engine
/// round trip at a time, and answers the engine's own cast-mode question with
/// the way the player already chose.
///
/// Returns how many times the engine asked which way — which is the number
/// that was **zero** before this repair.
fn play_it_out(duel: &mut baylee_client::Duel, table: &mut Table) -> usize {
    let mut asked = 0;
    for _ in 0..24 {
        refresh(duel, table);
        if matches!(table.pending, Some(Pending::ChooseCastMode { .. })) {
            asked += 1;
        }
        baylee_client::advance_mana_run(duel);
        baylee_client::take_the_chosen_cast_mode(duel);
        let sent = duel.take_outbox();
        if sent.is_empty() {
            break;
        }
        for action in sent {
            table.submit(action);
        }
    }
    assert_eq!(duel.last_error, None, "nothing aborted on the way through");
    asked
}
