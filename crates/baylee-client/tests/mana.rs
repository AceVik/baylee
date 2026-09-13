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
    let options = abilities::options(
        baylee_client_core::Lang::En,
        table.view(),
        &interaction,
        forest,
    );
    assert_eq!(options.len(), 1, "{options:?}");
    assert_eq!(options[0].label, "Tap for {G}");

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
    assert!(
        abilities::options(
            baylee_client_core::Lang::En,
            table.view(),
            &interaction,
            in_hand
        )
        .is_empty()
    );
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

/// A permanent whose ability is not a mana ability at all.
///
/// The label is the part worth pinning. `abilities::options` could only ever
/// say "Ability 1" for one of these, which is a label a player has to count
/// out on the card — and the chooser it is drawn into exists precisely so
/// they do not have to.
#[test]
fn a_non_mana_ability_is_named_by_what_it_costs() {
    use baylee_client::abilities;
    use baylee_client_core::interaction::Interaction;

    let mut table = Table::open_with(&preset_with_a_fetchland());
    table.walk_to_main();

    let interaction = Interaction::new(table.pending.clone().expect("priority"), PlayerId::new(0));
    let mire = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name == "Bloodstained Mire")
        .expect("the fetchland is on the table")
        .id;

    let options = abilities::options(
        baylee_client_core::Lang::En,
        table.view(),
        &interaction,
        mire,
    );
    assert_eq!(options.len(), 1, "{options:?}");
    assert_eq!(options[0].label, "{T}, Sacrifice this, Pay 1 life");
    assert_eq!(
        options[0].action,
        PlayerAction::ActivateAbility {
            source: mire,
            ability_index: 0,
        }
    );
}

/// …and the client's own click path activates it.
///
/// Through `activate_card`, which is where a pointer and the keyboard cursor
/// both end up: one option activates on the click that found it, so a player
/// never sees a menu of one. Before any of this the same click selected the
/// permanent for a choice that was not pending and did nothing at all.
#[test]
fn clicking_a_permanent_with_one_ability_activates_it() {
    use baylee_client::Duel;
    use baylee_client::input::activate_card;
    use baylee_client_core::interaction::Interaction;

    let mut table = Table::open_with(&preset_with_a_fetchland());
    table.walk_to_main();

    let mire = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name == "Bloodstained Mire")
        .expect("the fetchland is on the table")
        .id;
    let life = table.view().seat(PlayerId::new(0)).expect("own seat").life;

    let mut duel = Duel::default();
    duel.view = Some(table.view().clone());
    duel.interaction = Some(Interaction::new(
        table.pending.clone().expect("priority"),
        PlayerId::new(0),
    ));

    activate_card(&mut duel, mire);
    assert!(
        duel.ability_menu.is_none(),
        "one option needs no chooser at all"
    );
    // Sacrificing a land and paying a life is irreversible, so the first
    // click arms and the second sends (`docs/design.md` §2.5). This is the
    // ability the design names as the reason the rule exists.
    assert!(
        duel.outbox().is_empty(),
        "the first click sacrificed the land"
    );
    activate_card(&mut duel, mire);
    let action = duel.outbox().first().cloned().expect("the click sent one");
    assert_eq!(
        action,
        PlayerAction::ActivateAbility {
            source: mire,
            ability_index: 0,
        }
    );

    // The engine agrees, which is the half a client cannot fake: the land
    // sacrifices itself and the life is paid.
    table.submit(action);
    assert!(
        !table
            .view()
            .battlefield
            .iter()
            .any(|o| o.name == "Bloodstained Mire"),
        "the fetchland sacrificed itself"
    );
    assert_eq!(
        table.view().seat(PlayerId::new(0)).expect("own seat").life,
        life - 1,
        "and the life was paid"
    );
}

/// Yavimaya Coast — a painland, and therefore a permanent with two mana
/// abilities: `{T}: Add {C}` and `{T}: Add {G} or {U}` for a life.
const PAINLAND: &str = "40b36bc6-c185-4bda-99e7-0118953c2c97";

fn preset_with_a_painland() -> GamePreset {
    let mut preset = preset();
    preset.seats[0].starting_battlefield.push(entry(PAINLAND));
    preset
}

/// The ability chooser can be answered without a pointer.
///
/// It could not, and that is the whole reason this test exists: opening the
/// menu on a permanent with two abilities put the keyboard in a room with one
/// door, `Esc`. Confirm did nothing, the cursor keys walked the table behind
/// the menu, and the only way to activate the second ability was the mouse —
/// which breaks the keymap's promise that every choice is answerable without
/// one.
#[test]
fn the_ability_chooser_answers_to_the_keyboard() {
    use baylee_client::Duel;
    use baylee_client::input::{ability_menu_keys, activate_card};
    use baylee_client::keys::Fired;
    use baylee_client_core::interaction::Interaction;
    use baylee_client_core::prefs::Action;

    let mut table = Table::open_with(&preset_with_a_painland());
    table.walk_to_main();

    let coast = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name == "Yavimaya Coast")
        .expect("the painland is on the table")
        .id;

    let mut duel = Duel::default();
    duel.view = Some(table.view().clone());
    duel.interaction = Some(Interaction::new(
        table.pending.clone().expect("priority"),
        PlayerId::new(0),
    ));

    activate_card(&mut duel, coast);
    assert_eq!(
        duel.ability_menu,
        Some(coast),
        "two abilities are a menu, not a guess"
    );
    assert_eq!(duel.ability_pick, 0, "a fresh menu starts at the top");

    // The cursor walks it, and wraps rather than sticking at the end.
    assert!(ability_menu_keys(
        Fired::of_actions(&[Action::CursorDown]),
        &mut duel
    ));
    assert_eq!(duel.ability_pick, 1);
    assert!(ability_menu_keys(
        Fired::of_actions(&[Action::CursorDown]),
        &mut duel
    ));
    assert_eq!(duel.ability_pick, 0, "the list is a ring");

    // And confirm sends the entry the highlight is on — the second one,
    // which is the one a pointer used to be needed for.
    assert!(ability_menu_keys(
        Fired::of_actions(&[Action::CursorUp]),
        &mut duel
    ));
    assert_eq!(duel.ability_pick, 1);
    assert!(ability_menu_keys(
        Fired::of_actions(&[Action::Confirm]),
        &mut duel
    ));
    assert!(duel.ability_menu.is_none(), "answering closes the menu");
    assert_eq!(
        duel.outbox().first().cloned().expect("the key sent one"),
        PlayerAction::ActivateAbility {
            source: coast,
            ability_index: 1,
        }
    );
}

/// Cancel puts the menu away and sends nothing.
#[test]
fn cancelling_the_ability_chooser_activates_nothing() {
    use baylee_client::Duel;
    use baylee_client::input::{ability_menu_keys, activate_card};
    use baylee_client::keys::Fired;
    use baylee_client_core::interaction::Interaction;
    use baylee_client_core::prefs::Action;

    let mut table = Table::open_with(&preset_with_a_painland());
    table.walk_to_main();

    let coast = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name == "Yavimaya Coast")
        .expect("the painland is on the table")
        .id;

    let mut duel = Duel::default();
    duel.view = Some(table.view().clone());
    duel.interaction = Some(Interaction::new(
        table.pending.clone().expect("priority"),
        PlayerId::new(0),
    ));
    activate_card(&mut duel, coast);
    assert!(ability_menu_keys(
        Fired::of_actions(&[Action::Cancel]),
        &mut duel
    ));
    assert!(duel.ability_menu.is_none());
    assert!(duel.outbox().is_empty(), "cancel is not an answer");
}

/// Karn, the Great Creator — a planeswalker, and therefore a permanent with
/// two abilities *neither* of which makes mana.
const WALKER: &str = "a20dd48d-d344-4db1-b0e9-a2b71c3cc9d1";

fn preset_with_a_planeswalker() -> GamePreset {
    let mut preset = preset();
    preset.seats[0].starting_battlefield.push(entry(WALKER));
    preset
}

/// Picking from the ability sheet *arms*; it does not send, and the sheet
/// stays open while it is armed.
///
/// The sheet disambiguates and confirming is a separate statement, which is
/// the whole of arm-then-act. The painland test above cannot show it: both of
/// Yavimaya Coast's abilities make mana, and a mana ability is the one
/// exemption, so that path goes straight onto the wire. A planeswalker is the
/// other shape — two abilities, neither of them mana — and it is the one this
/// branch actually runs in.
///
/// Staying open is the half the sheet added. The row's keycap goes gilt and
/// the footer says to press the same digit again, so a sheet that closed on
/// the arming would take away the only key it had just named — and the two
/// states have two ways out, the first escape disarming and the second
/// closing.
#[test]
fn picking_from_the_chooser_arms_rather_than_sends() {
    use baylee_client::input::{ability_menu_keys, activate_card, armed_keys};
    use baylee_client::keys::Fired;
    use baylee_client::{Deed, Duel};
    use baylee_client_core::interaction::Interaction;
    use baylee_client_core::prefs::Action;

    let mut table = Table::open_with(&preset_with_a_planeswalker());
    table.walk_to_main();

    let karn = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name.starts_with("Karn"))
        .expect("the planeswalker is on the table")
        .id;

    let mut duel = Duel::default();
    duel.view = Some(table.view().clone());
    duel.interaction = Some(Interaction::new(
        table.pending.clone().expect("priority"),
        PlayerId::new(0),
    ));

    let options = baylee_client::abilities::options(
        baylee_client_core::Lang::En,
        duel.view.as_ref().expect("a view"),
        duel.interaction.as_ref().expect("a choice"),
        karn,
    );
    assert!(
        options.len() >= 2,
        "a loyalty ability is not a mana ability, so both are offered: {options:?}"
    );
    assert!(
        options.iter().all(|o| !o.mana),
        "none of a planeswalker's abilities makes mana"
    );

    activate_card(&mut duel, karn);
    assert_eq!(duel.ability_menu, Some(karn), "two abilities are a menu");

    assert!(ability_menu_keys(
        Fired::of_actions(&[Action::CursorDown]),
        &mut duel
    ));
    assert!(ability_menu_keys(
        Fired::of_actions(&[Action::Confirm]),
        &mut duel
    ));
    assert_eq!(
        duel.ability_menu,
        Some(karn),
        "the sheet stays open while a row is armed: the digit that armed it \
         is the digit that sends it, and a sheet that closed here would take \
         that digit away"
    );
    assert!(
        duel.outbox().is_empty(),
        "picking is not confirming — nothing is on the wire yet"
    );
    assert_eq!(
        duel.armed.as_ref().map(|a| a.deed.clone()),
        Some(Deed::Ability(options[1].action.clone())),
        "what is armed is the entry the highlight was on"
    );

    // Escape takes the arming back and leaves the sheet standing, because
    // those are two states and there are two ways out of them.
    assert!(armed_keys(Fired::of_actions(&[Action::Cancel]), &mut duel));
    assert!(duel.armed.is_none(), "the first escape disarms");
    assert_eq!(duel.ability_menu, Some(karn), "and leaves the sheet open");
    assert!(ability_menu_keys(
        Fired::of_actions(&[Action::Cancel]),
        &mut duel
    ));
    assert!(duel.ability_menu.is_none(), "the second closes it");

    // Arm it again, and the confirm after it is the send.
    duel.ability_menu = Some(karn);
    duel.ability_pick = 1;
    assert!(ability_menu_keys(
        Fired::of_actions(&[Action::Confirm]),
        &mut duel
    ));
    assert!(armed_keys(Fired::of_actions(&[Action::Confirm]), &mut duel));
    assert_eq!(
        duel.outbox().first().cloned().expect("the key sent one"),
        options[1].action,
    );
    assert!(duel.armed.is_none(), "sending disarms");
    assert!(
        duel.ability_menu.is_none(),
        "and the sheet has been answered"
    );
}

/// The digit drawn on a row arms that row, and the same digit sends it.
///
/// This is the two-stage mechanic reached the way a player reaches it, and
/// the three things it has bitten on before are all here: the arming has to
/// *move* when a second row is pressed rather than adding a second armed
/// deed, the send has to go through `fire_armed` so the engine's current
/// offer is what is sent, and the sheet has to stay open in between —
/// because the digit that armed the row is the digit that sends it, and a
/// sheet that closed on the arming would take that digit away.
#[test]
fn a_digit_arms_the_row_it_is_drawn_on_and_the_same_digit_sends_it() {
    use baylee_client::input::{activate_card, sheet_digit};
    use baylee_client::{Deed, Duel};
    use baylee_client_core::interaction::Interaction;

    let mut table = Table::open_with(&preset_with_a_planeswalker());
    table.walk_to_main();

    let karn = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name.starts_with("Karn"))
        .expect("the planeswalker is on the table")
        .id;

    let mut duel = Duel::default();
    duel.view = Some(table.view().clone());
    duel.interaction = Some(Interaction::new(
        table.pending.clone().expect("priority"),
        PlayerId::new(0),
    ));

    let options = baylee_client::abilities::options(
        baylee_client_core::Lang::En,
        duel.view.as_ref().expect("a view"),
        duel.interaction.as_ref().expect("a choice"),
        karn,
    );
    assert!(options.len() >= 2, "two rows to tell apart: {options:?}");
    assert!(
        options.len() <= baylee_client_core::abilitysheet::PAGE,
        "one page, so `0` is a key the sheet never drew: {options:?}"
    );

    activate_card(&mut duel, karn);
    assert_eq!(duel.ability_menu, Some(karn), "two abilities are a sheet");

    assert!(sheet_digit(&mut duel, '2'), "the second row wears a `2`");
    assert_eq!(duel.ability_pick, 1, "and the cursor goes to it");
    assert_eq!(
        duel.armed.as_ref().map(|a| a.deed.clone()),
        Some(Deed::Ability(options[1].action.clone())),
        "a loyalty ability costs something, so the digit arms it"
    );
    assert_eq!(duel.ability_menu, Some(karn), "the sheet stays open");
    assert!(duel.outbox().is_empty(), "nothing is on the wire yet");

    // A different digit moves the arming rather than adding to it: there is
    // one `Duel::armed`, and a player changing their mind is the ordinary
    // case rather than an error.
    assert!(sheet_digit(&mut duel, '1'), "the first row wears a `1`");
    assert_eq!(
        duel.armed.as_ref().map(|a| a.deed.clone()),
        Some(Deed::Ability(options[0].action.clone())),
        "the arming moved"
    );
    assert!(duel.outbox().is_empty(), "and still nothing was sent");

    // A key nothing drew. The pager is the tenth row and this sheet has two.
    let before = (duel.ability_page, duel.ability_pick);
    assert!(
        !sheet_digit(&mut duel, '0'),
        "`0` is the pager and there is no second page"
    );
    assert_eq!(
        (duel.ability_page, duel.ability_pick),
        before,
        "so it moved nothing"
    );
    assert!(duel.outbox().is_empty(), "and sent nothing");

    // The same digit again is the send.
    assert!(
        sheet_digit(&mut duel, '1'),
        "the row is pressed a second time"
    );
    assert_eq!(
        duel.outbox().first().cloned().expect("the digit sent one"),
        options[0].action,
    );
    assert!(duel.armed.is_none(), "sending disarms");
    assert!(
        duel.ability_menu.is_none(),
        "and the sheet has been answered"
    );
}

/// An armed run can come back a different answer, and then it is a cast.
///
/// This is the one arming path that re-resolves to something *other* than
/// itself. Between the two taps this seat holds priority, so the board can
/// only have been changed by this seat — and the one thing it can have done
/// is tap a land by hand. Once that pays the cost the engine offers the spell
/// outright, and a run started anyway would tap two more lands and float mana
/// nobody asked for.
///
/// The ordering is what the test is really about: `play_card` is read
/// *before* `reachable`, so `duel.reachable` is deliberately left saying yes
/// here. What the client offered to do a moment ago must not outvote what the
/// engine is offering now.
#[test]
fn an_armed_run_becomes_a_plain_cast_when_the_lands_are_tapped_by_hand() {
    use baylee_client::input::{activate_card, armed_keys};
    use baylee_client::keys::Fired;
    use baylee_client::{Deed, Duel};
    use baylee_client_core::interaction::Interaction;
    use baylee_client_core::prefs::Action;

    let mut table = Table::open();
    table.walk_to_main();

    let spell = table
        .view()
        .hand
        .iter()
        .find(|c| c.name == "Great Divide Guide")
        .expect("the creature is in the opening hand")
        .id;

    let mut duel = Duel::default();
    duel.view = Some(table.view().clone());
    duel.interaction = Some(Interaction::new(
        table.pending.clone().expect("priority"),
        PlayerId::new(0),
    ));
    duel.reachable = std::iter::once(spell).collect();

    activate_card(&mut duel, spell);
    let Some(Deed::Run { plan, .. }) = duel.armed.as_ref().map(|a| a.deed.clone()) else {
        panic!("a spell the lands can pay for arms a run: {:?}", duel.armed);
    };
    assert!(duel.outbox().is_empty(), "arming puts nothing on the wire");

    // The player pays it themselves instead, which is the thing that was
    // always allowed and is the only way the board can move here.
    for step in &plan.steps {
        table.submit(PlayerAction::ActivateManaAbility {
            source: step.source,
        });
    }
    assert!(
        table.legal().castable.contains(&spell),
        "with the mana floating the engine offers the spell"
    );
    duel.view = Some(table.view().clone());
    duel.interaction = Some(Interaction::new(
        table.pending.clone().expect("priority"),
        PlayerId::new(0),
    ));

    // Confirm now casts. No run is started, and no further land is tapped.
    assert!(armed_keys(Fired::of_actions(&[Action::Confirm]), &mut duel));
    assert_eq!(
        duel.outbox().first().cloned().expect("the key sent one"),
        PlayerAction::CastSpell { card: spell },
        "the armed run resolved to the cast it was standing in for"
    );
    assert!(duel.mana_run.is_none(), "nothing was tapped for it");
    assert!(duel.armed.is_none(), "sending disarms");
}

/// Chromatic Lantern — "Lands you control have `{T}: Add one mana of any
/// color`". The card that turns a fetchland into a mana source in the eyes of
/// `LegalActions`, and therefore the card that broke the click path.
const LANTERN: &str = "539f5396-d99a-417d-a84c-dff7930b5900";

/// Sea Gate Loremaster — `{T}: Draw a card for each Ally you control`. The
/// shape the owner named: a cost paid entirely by tapping the permanent, and
/// nothing else.
const LOREMASTER: &str = "6eed122b-9760-47fd-8ba2-adeda8054e0d";

/// The fetchland bug, end to end.
///
/// Reported from a live game: "Fetchland Effekt fügt irgendwas in den Manapool
/// statt mich ein passendes Land aus der Bibliothek auswählen zu lassen."
///
/// The card was never the fault. `Interaction::activate` routed on the ability
/// index's *numeric value* — a permanent named in `mana_abilities` plus an
/// index of 0 meant "mana ability", whatever the engine had offered there. The
/// Lantern grants every land a mana ability, and Bloodstained Mire's real
/// ability sits at index 0. Both cards are in the starter deck the lobby
/// posts, which is why this was met in the first game.
///
/// Driven through `activate_card` rather than through `Interaction`, because
/// what is claimed is about a *click*: an assertion on the resolver would pass
/// just as well if no click could reach it.
#[test]
fn a_fetchland_under_a_chromatic_lantern_still_searches() {
    use baylee_client::Duel;
    use baylee_client::input::{ability_menu_keys, activate_card, armed_keys};
    use baylee_client::keys::Fired;
    use baylee_client_core::interaction::Interaction;
    use baylee_client_core::prefs::Action;

    let mut preset = preset();
    preset.seats[0].starting_battlefield.push(entry(MIRE));
    preset.seats[0].starting_battlefield.push(entry(LANTERN));
    let mut table = Table::open_with(&preset);
    table.walk_to_main();

    let mire = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name == "Bloodstained Mire")
        .expect("the fetchland is on the table")
        .id;

    // The precondition, so the test cannot pass by the grant never arriving:
    // the engine must be naming the Mire in *both* lists at once.
    let legal = match table.pending.as_ref().expect("priority") {
        baylee_engine::choice::Pending::Priority { legal, .. } => legal.clone(),
        other => panic!("not a priority window: {other:?}"),
    };
    assert!(
        legal.mana_abilities.contains(&mire),
        "the Lantern did not grant the fetchland a mana ability"
    );
    assert!(
        legal.abilities.contains(&(mire, 0)),
        "the fetchland's own ability is not at index 0"
    );

    let mut duel = Duel::default();
    duel.view = Some(table.view().clone());
    duel.interaction = Some(Interaction::new(
        table.pending.clone().expect("priority"),
        PlayerId::new(0),
    ));

    // The list is where the bug was visible: `Interaction::activate` turned
    // index 0 into a mana ability, so the search was not in it at all.
    let options = baylee_client::abilities::options(
        baylee_client_core::Lang::En,
        duel.view.as_ref().expect("a view"),
        duel.interaction.as_ref().expect("a choice"),
        mire,
    );
    let search = options
        .iter()
        .position(|o| {
            o.action
                == PlayerAction::ActivateAbility {
                    source: mire,
                    ability_index: 0,
                }
        })
        .unwrap_or_else(|| panic!("the search is not on offer at all: {options:?}"));
    assert!(
        !options[search].mana,
        "the fetch was read as a mana ability"
    );
    assert!(
        !options[search].tap_only,
        "a sacrifice and a life are not paid out of the card"
    );

    // And the click path reaches it. The Lantern gives the land a second,
    // granted ability, so this is now honestly a menu of two — which is the
    // right answer and was never the reported one: before this, the click
    // silently tapped for mana.
    activate_card(&mut duel, mire);
    assert_eq!(
        duel.ability_menu,
        Some(mire),
        "two abilities on one land are a menu"
    );
    for _ in 0..search {
        assert!(ability_menu_keys(
            Fired::of_actions(&[Action::CursorDown]),
            &mut duel
        ));
    }
    assert!(ability_menu_keys(
        Fired::of_actions(&[Action::Confirm]),
        &mut duel
    ));
    // Armed, not sent: sacrificing a land and paying a life is irreversible.
    assert!(duel.outbox().is_empty(), "picking sacrificed the land");
    assert!(armed_keys(Fired::of_actions(&[Action::Confirm]), &mut duel));
    assert_eq!(
        duel.outbox(),
        [PlayerAction::ActivateAbility {
            source: mire,
            ability_index: 0,
        }],
        "the fetchland tapped for mana instead of searching"
    );
}

/// A `{T}: …` ability with no other cost fires on one tap.
///
/// The owner's words: "Ich möchte z.B. tap zum ziehen sagen, die Karte, der
/// Effekt und das wars." The line that lets it is not "mana abilities are
/// special" but the reason mana abilities were special in the first place —
/// the whole cost comes out of the card itself and the next untap step gives
/// it back.
#[test]
fn a_tap_only_ability_fires_on_one_tap() {
    use baylee_client::Duel;
    use baylee_client::abilities;
    use baylee_client::input::activate_card;
    use baylee_client_core::interaction::Interaction;

    let mut preset = preset();
    preset.seats[0].starting_battlefield.push(entry(LOREMASTER));
    let mut table = Table::open_with(&preset);
    table.walk_to_main();

    let loremaster = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name == "Sea Gate Loremaster")
        .expect("the Loremaster is on the table")
        .id;

    let interaction = Interaction::new(table.pending.clone().expect("priority"), PlayerId::new(0));
    let options = abilities::options(
        baylee_client_core::Lang::En,
        table.view(),
        &interaction,
        loremaster,
    );
    assert_eq!(options.len(), 1, "{options:?}");
    assert!(!options[0].mana, "drawing cards is not a mana ability");
    assert!(options[0].tap_only, "{{T}} alone was not read as tap-only");

    let mut duel = Duel::default();
    duel.view = Some(table.view().clone());
    duel.interaction = Some(interaction);
    activate_card(&mut duel, loremaster);
    assert!(
        duel.armed.is_none(),
        "tapping to draw asked for a confirmation"
    );
    assert_eq!(
        duel.outbox(),
        [PlayerAction::ActivateAbility {
            source: loremaster,
            ability_index: 0,
        }]
    );
}

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

/// Suspending, end to end, the way the owner asked for it.
///
/// Reported as: *"Suspend works different. The text says: Rather then cast
/// this spell from your hand, PAY x and exile …, so first tap and pay mana,
/// then suspend."* Which is what the card says, and neither half of this
/// client did it: the engine offered `suspendable` off an empty pool (fixed
/// on its side), and nothing here ever built a `PlayerAction::Suspend` at
/// all — `legal.suspendable` was read by the automation rules and by nothing
/// else, so a Suspend 4—{U} in hand answered no click.
///
/// Driven through `activate_card` and `advance_mana_run` against a real
/// `LocalHost`, because what is claimed is about a *click*: a test that built
/// the action by hand would pass just as loudly with no button behind it.
#[test]
fn a_suspend_card_taps_its_island_and_then_suspends() {
    use baylee_client::input::activate_card;
    use baylee_client::{Deed, Duel, RunEnd, advance_mana_run};
    use baylee_client_core::interaction::Interaction;

    let mut table = Table::open_with(&suspend_preset());
    table.walk_to_main();

    let vision = table
        .view()
        .hand
        .iter()
        .find(|c| c.name == "Ancestral Vision")
        .expect("the card is in the opening hand")
        .id;

    // The engine offers neither half of it yet, which is the whole problem:
    // the {U} is still in the Island. And it is not castable at any point —
    // a card with no mana cost cannot be cast (CR 202.1a).
    assert!(
        !table.legal().suspendable.contains(&vision),
        "the cost is not floating, so the engine offers no suspend"
    );
    assert!(
        !table.legal().castable.contains(&vision),
        "and a blank mana cost is never a free spell"
    );

    let mut duel = Duel::default();
    let refresh = |duel: &mut Duel, table: &Table| {
        duel.view = Some(table.view().clone());
        duel.interaction = Some(Interaction::new(
            table.pending.clone().expect("priority"),
            PlayerId::new(0),
        ));
        baylee_client::rebuild_board(duel);
    };
    refresh(&mut duel, &table);
    assert!(
        duel.suspend_reach.contains(&vision),
        "one untapped Island pays {{U}}, so the client offers to tap it"
    );

    // And the drawn hand says so, which is the half a player can actually
    // see. A suspend-only card used to sit there with no light on it at all:
    // `Openings` was built from `lands` and `castable` alone, so neither the
    // engine's own `suspendable` nor this client's `suspend_reach` reached
    // the board model. Clicking it worked — finding it did not.
    let drawn = |duel: &Duel| {
        duel.board
            .as_ref()
            .expect("a view builds a board")
            .hand
            .iter()
            .find(|c| c.id == vision)
            .map(|c| (c.playable, c.reachable))
            .expect("the card is in the drawn hand")
    };
    assert_eq!(
        drawn(&duel),
        (false, true),
        "with the {{U}} still in the Island: indigo, and not gold"
    );

    // First click arms, second click sends. Nothing reaches the wire in
    // between — suspending exiles the card and there is no undo.
    activate_card(&mut duel, vision);
    let Some(Deed::Run { then, .. }) = duel.armed.as_ref().map(|a| a.deed.clone()) else {
        panic!("the click arms a run: {:?}", duel.armed);
    };
    assert_eq!(then, RunEnd::Suspend, "and the run ends in a suspend");
    assert!(duel.outbox().is_empty(), "arming puts nothing on the wire");

    activate_card(&mut duel, vision);
    for action in duel.take_outbox() {
        table.submit(action);
    }
    assert!(duel.mana_run.is_some(), "the second click starts the run");

    // Now spend it, one engine round trip per step, exactly as the frame
    // loop does.
    let mut gold_once_the_mana_was_up = false;
    for _ in 0..8 {
        refresh(&mut duel, &table);
        // The other half of the light: the moment the engine itself offers
        // the suspend, the card is gold rather than indigo — the same
        // promotion a castable spell gets when its mana floats.
        if table.legal().suspendable.contains(&vision) {
            gold_once_the_mana_was_up = drawn(&duel).0;
        }
        advance_mana_run(&mut duel);
        let sent = duel.take_outbox();
        if sent.is_empty() {
            break;
        }
        for action in sent {
            table.submit(action);
        }
        if duel.mana_run.is_none() {
            break;
        }
    }
    assert_eq!(duel.last_error, None, "the run finished without aborting");
    assert!(
        gold_once_the_mana_was_up,
        "the engine's own offer lights the card gold"
    );

    // The card is in exile with four time counters on it, and the Island is
    // tapped. That is what "pay {U} and exile it" means.
    let exiled = table
        .view()
        .exile
        .iter()
        .flatten()
        .find(|o| o.name == "Ancestral Vision")
        .expect("the suspended card is in exile");
    assert_eq!(
        exiled
            .counters
            .iter()
            .find(|c| c.kind == baylee_view::CounterKind::Time)
            .map(|c| c.count),
        Some(4),
        "Suspend 4 exiles it with four time counters: {:?}",
        exiled.counters
    );
}

/// The click path resolves a cast before a suspend, and nothing in the pool
/// is both.
///
/// `activate_card` reads `play_card` first and `suspend` after it, which is
/// an *order* and would be a silent choice on a card that offered both — and
/// there is no undo for either. The pool makes the question moot: a suspend
/// card prints no mana cost, so CR 202.1a keeps it out of `castable`
/// altogether. This is that claim as a build failure rather than a comment,
/// because the first card that breaks it needs a chooser and not an order.
#[test]
fn no_suspend_card_in_the_pool_is_also_castable() {
    let mut both: Vec<&str> = Vec::new();
    for def in baylee_cards::all() {
        let suspends = def
            .abilities
            .iter()
            .any(|a| matches!(a, baylee_cards_dsl::AbilityDef::Suspend { .. }));
        if !suspends {
            continue;
        }
        for face in def.faces {
            if face.mana_cost.symbols().next().is_some() {
                both.push(face.name);
            }
        }
    }
    assert!(
        both.is_empty(),
        "these cards can be cast *and* suspended, so one click has two \
         answers and `activate_card` picks the first silently: {both:?}"
    );
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

/// The owner's fourth point, end to end: **the card taps only once the mana
/// has been chosen.**
///
/// Reported as *"Bei Karten die reines Mana generieren (aber halt Wahl des
/// Spielers). Da sollte lieber so ein kleines Pergament-Stil Dialog direkt
/// unter der Karte aufploppen wo der Spieler dann das Mana-Symbol wählen
/// kann"*, and then, in the same breath: *"die Karte wird erst dann getappt,
/// wenn das Mana ausgewählt wurde"*.
///
/// What it replaces is the one-option short-circuit in `activate_card`: a
/// Tundra offered exactly one thing, so the click fired it, the land tapped,
/// and the colour was asked afterwards in the prompt bar at the bottom of the
/// screen. The assertion that carries the whole report is the one in the
/// middle — the land is **untapped** while the bubble stands open.
#[test]
fn a_land_that_asks_which_colour_is_not_tapped_until_the_answer() {
    use baylee_client::input::{activate_card, sheet_digit};
    use baylee_client::{Duel, advance_mana_run};
    use baylee_client_core::interaction::Interaction;

    let mut table = Table::open_with(&bubble_preset());
    table.walk_to_main();

    let tundra = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name == "Tundra")
        .expect("the land starts on the table")
        .id;

    let mut duel = Duel::default();
    let refresh = |duel: &mut Duel, table: &Table| {
        duel.view = Some(table.view().clone());
        duel.interaction = Some(Interaction::new(
            table.pending.clone().expect("priority"),
            PlayerId::new(0),
        ));
        baylee_client::rebuild_board(duel);
    };
    let tapped = |table: &Table| {
        table
            .view()
            .battlefield
            .iter()
            .find(|o| o.id == tundra)
            .expect("still on the table")
            .status
            .contains(baylee_view::ObjectStatus::TAPPED)
    };
    refresh(&mut duel, &table);
    assert!(!tapped(&table), "it starts untapped");

    // The click opens the bubble and sends nothing. Two pips, because a
    // Tundra makes two colours.
    activate_card(&mut duel, tundra);
    assert_eq!(duel.ability_menu, Some(tundra), "the bubble is open");
    assert!(duel.outbox().is_empty(), "and nothing reached the wire");
    assert!(!tapped(&table), "**the land is still untapped**");

    let options = baylee_client::abilities::options(
        baylee_client_core::Lang::En,
        duel.view.as_ref().expect("a view"),
        duel.interaction.as_ref().expect("priority"),
        tundra,
    );
    assert!(
        baylee_client::abilities::pouring(&options),
        "every row is a pour, so the sheet draws pips: {options:?}"
    );
    assert_eq!(
        options
            .iter()
            .filter_map(|o| o.pour.map(|p| p.color))
            .collect::<Vec<_>>(),
        vec![
            baylee_core::mana::ManaColor::White,
            baylee_core::mana::ManaColor::Blue
        ],
        "white then blue, in `ManaColor` order"
    );

    // The second pip: blue. One press, and the activation goes out with the
    // colour already decided.
    assert!(sheet_digit(&mut duel, '2'), "the digit names a pip");
    assert!(duel.mana_run.is_some(), "which starts a one-step run");
    assert_eq!(duel.ability_menu, None, "and puts the bubble away");

    for _ in 0..8 {
        for action in duel.take_outbox() {
            table.submit(action);
        }
        refresh(&mut duel, &table);
        if duel.mana_run.is_none() {
            break;
        }
        advance_mana_run(&mut duel);
    }

    assert_eq!(duel.last_error, None, "the run finished without aborting");
    assert!(duel.mana_run.is_none(), "and finished");
    assert!(tapped(&table), "*now* the land is tapped");
    let pool = table
        .view()
        .seat(PlayerId::new(0))
        .expect("own seat")
        .mana_pool;
    assert_eq!(
        (pool.blue, pool.white),
        (1, 0),
        "one blue, which is the pip that was pressed: {pool:?}"
    );
    assert!(
        table.view().stack.is_empty(),
        "a mana ability never uses the stack (CR 605.3a)"
    );
}

/// The same bubble on a creature, and on the ability no planner can read.
///
/// Harabaz Druid's `Add X mana of any one color` has an `Amount::CountOf`,
/// so `mana_shape` refuses it and `manasources::sources` has never seen it —
/// it was the empty `↺` row on the sheet. `mana_offer` reads the colour
/// question alone, which is all a bubble asks, and the owner's addendum said
/// the dialog is for *"Artefakte und Kreaturen, die nur Mana Ability haben"*.
#[test]
fn a_creature_whose_only_ability_is_mana_gets_the_same_five_pips() {
    use baylee_client::input::activate_card;
    use baylee_client::{Duel, abilities, manasources};
    use baylee_client_core::interaction::Interaction;
    use baylee_core::mana::ManaColor;

    let mut table = Table::open_with(&bubble_preset());
    table.walk_to_main();

    let druid = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name == "Harabaz Druid")
        .expect("the creature starts on the table")
        .id;

    // The premise, and the reason this needed a fourth reading at all: the
    // planner cannot count what this makes, so it is in no `Source` list.
    assert!(
        !manasources::sources(table.view(), table.legal())
            .iter()
            .any(|s| s.id == druid),
        "no planner can read an amount that is a count of the battlefield"
    );

    let mut duel = Duel::default();
    duel.view = Some(table.view().clone());
    duel.interaction = Some(Interaction::new(
        table.pending.clone().expect("priority"),
        PlayerId::new(0),
    ));
    baylee_client::rebuild_board(&mut duel);

    let options = abilities::options(
        baylee_client_core::Lang::En,
        duel.view.as_ref().expect("a view"),
        duel.interaction.as_ref().expect("priority"),
        druid,
    );
    assert!(abilities::pouring(&options), "a bubble: {options:?}");
    assert_eq!(
        options
            .iter()
            .filter_map(|o| o.pour.map(|p| p.color))
            .collect::<Vec<_>>(),
        vec![
            ManaColor::White,
            ManaColor::Blue,
            ManaColor::Black,
            ManaColor::Red,
            ManaColor::Green
        ],
        "any one colour, in WUBRG order"
    );

    activate_card(&mut duel, druid);
    assert_eq!(duel.ability_menu, Some(druid), "the click opens the bubble");
    assert!(duel.outbox().is_empty(), "and taps nothing");
}

/// The owner's sixth point: a permanent that makes mana **and** does
/// something else keeps its sheet, with the colours as a header on it.
///
/// Reported as *"Wenn es sowas wie Harabaz Druid in der Kombi ist, oder wie
/// beim Gates of Istfell: Dann kommt der normale Abilities Dialog ABER er hat
/// eine Besonderheit. Die erste Zeile ist quasi sowas wie der Mana-Dialog,
/// dort stehen zentriert die Farben, die es produzieren kann (oder auch
/// Farblos wie beim Artefakt) und via Klick ist es dann das. Nur dass man eben
/// auch die anderen Abilities noch zur Auswahl hat."*
///
/// Two halves, and the second is the one that could quietly go wrong: the pips
/// carry **no digit**, so `1` is the written row and not the `{C}`. A list that
/// numbered the pips would put a `2` on the only keycap this sheet draws.
#[test]
fn a_permanent_that_also_makes_mana_gets_the_pips_as_a_header() {
    use baylee_client::input::{activate_card, sheet_digit};
    use baylee_client::{Duel, abilities};
    use baylee_client_core::interaction::Interaction;

    let mut preset = bubble_preset();
    preset.seats[0].starting_battlefield = vec![entry(HOMEWARD)];
    let mut table = Table::open_with(&preset);
    table.walk_to_main();

    let path = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name == "Homeward Path")
        .expect("the land starts on the table")
        .id;

    let mut duel = Duel::default();
    duel.view = Some(table.view().clone());
    duel.interaction = Some(Interaction::new(
        table.pending.clone().expect("priority"),
        PlayerId::new(0),
    ));
    baylee_client::rebuild_board(&mut duel);

    let options = abilities::options(
        baylee_client_core::Lang::En,
        duel.view.as_ref().expect("a view"),
        duel.interaction.as_ref().expect("priority"),
        path,
    );
    assert!(
        !abilities::pouring(&options),
        "not a bubble — it does something besides make mana: {options:?}"
    );
    let split = abilities::Split::of(&options);
    assert_eq!(
        (split.pips, split.rows),
        (1, 1),
        "one pip for the {{C}}, one written row for the rest: {options:?}"
    );
    assert_eq!(
        options[0].pour.map(|pour| pour.color),
        Some(baylee_core::mana::ManaColor::Colorless),
        "and a colourless pip is a pip — `of_mana` answers what `of_color` \
         cannot (CR 106.1b)"
    );

    activate_card(&mut duel, path);
    assert_eq!(duel.ability_menu, Some(path), "the sheet opens");
    assert!(duel.outbox().is_empty(), "and sends nothing");

    // The one keycap on this sheet says `1`, and it is on the sentence. The
    // pip above it carries no digit at all.
    assert!(
        sheet_digit(&mut duel, '1'),
        "the digit names the written row"
    );
    assert!(duel.mana_run.is_none(), "which is not a pour");
    assert_eq!(
        duel.outbox(),
        [PlayerAction::ActivateAbility {
            source: path,
            ability_index: 1,
        }],
        "`1` sent the sentence, not the {{C}}"
    );
}

/// Round seven's first point: Harabaz Druid **under a Great Divide Guide**,
/// where it has two mana abilities and one of them had gone missing.
///
/// Reported as *"er hat in der Konstilazion zwei Mana Abilities. 1x die
/// geerbte vom Great Divide Guide und dann die eigene, mit X = count(allies),
/// diese ist gerade 'verloren gegangen'"*. Both taps make all five colours, so
/// the colour-wise union has one pip per colour to share between them and the
/// loser used to be dropped from the list outright.
///
/// Two claims, and they are the two halves of the fix. The pips stand for the
/// **grant**, because a pip promises one mana of the colour pressed and the
/// Druid's own X is a count of the battlefield nothing this side of the engine
/// reads. And the Druid's own ability is a written row saying what it makes,
/// because a tap the header does not stand for keeps its sentence.
#[test]
fn a_second_mana_tap_the_pips_cannot_stand_for_keeps_its_sentence() {
    use baylee_client::input::{activate_card, sheet_digit};
    use baylee_client::{Duel, abilities};
    use baylee_client_core::interaction::Interaction;
    use baylee_engine::choice::GRANTED_ABILITY;

    let mut preset = bubble_preset();
    preset.seats[0].starting_battlefield = vec![entry(SQUAD), entry(HARABAZ)];
    let mut table = Table::open_with(&preset);
    table.walk_to_main();

    let druid = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name == "Harabaz Druid")
        .expect("the creature starts on the table")
        .id;

    let mut duel = Duel::default();
    duel.view = Some(table.view().clone());
    duel.interaction = Some(Interaction::new(
        table.pending.clone().expect("priority"),
        PlayerId::new(0),
    ));
    baylee_client::rebuild_board(&mut duel);

    // The premise: the engine is offering both, the Druid's own ability at
    // index 0 and the Guide's grant under the synthetic one.
    let legal = table.legal();
    assert!(legal.abilities.contains(&(druid, 0)), "its own {legal:?}");
    assert!(
        legal.abilities.contains(&(druid, GRANTED_ABILITY)),
        "and the one the Guide hands every Ally: {legal:?}"
    );

    let options = abilities::options(
        baylee_client_core::Lang::En,
        duel.view.as_ref().expect("a view"),
        duel.interaction.as_ref().expect("priority"),
        druid,
    );
    let split = abilities::Split::of(&options);
    assert_eq!(
        (split.pips, split.rows),
        (5, 1),
        "five colours as a header, and the ability they are not: {options:?}"
    );
    assert!(
        options[..split.pips].iter().all(|option| option
            .pour
            .as_ref()
            .is_some_and(|pour| pour.step.tap == manaplan::Tap::Ability(GRANTED_ABILITY))),
        "every pip is the grant, whose pour is one mana: {options:?}"
    );
    assert_eq!(
        options[0].action,
        PlayerAction::ActivateAbility {
            source: druid,
            ability_index: GRANTED_ABILITY,
        },
        "and presses the activation the engine offered it under"
    );

    let own = &options[split.option(0)];
    assert!(own.pour.is_none(), "the written row is no pip: {own:?}");
    assert_eq!(
        own.action,
        PlayerAction::ActivateAbility {
            source: druid,
            ability_index: 0,
        },
        "and it is the Druid's own ability"
    );
    assert_eq!(
        own.label, "Tap for X mana (any color)",
        "which says what it makes rather than repeating its own cost"
    );
    assert_eq!(own.cost.as_deref(), Some("{T}"), "the cost column: {own:?}");
    // Round eight's second point: *"ich fände es schöner, wenn hier der
    // Original Text angezeigt wird, statt einem customizierten"*. The label
    // above is a fallback and is what this row drew until the line table
    // learned to place a mana ability at all; the Druid prints one sentence
    // and this is it.
    assert_eq!(
        own.printed,
        Some(baylee_view::StackText {
            face: 0,
            line: 0,
            of: 1,
        }),
        "the card's own sentence, not the label: {own:?}"
    );

    // And the one keycap this sheet draws is on that row, the pips carrying
    // no digit.
    activate_card(&mut duel, druid);
    assert_eq!(duel.ability_menu, Some(druid), "the click opens the sheet");
    assert!(duel.outbox().is_empty(), "and taps nothing");
    assert!(
        sheet_digit(&mut duel, '1'),
        "the digit names the written row"
    );
    assert!(duel.mana_run.is_none(), "which is not a pour");
    assert_eq!(
        duel.outbox(),
        [PlayerAction::ActivateAbility {
            source: druid,
            ability_index: 0,
        }],
        "`1` sent the ability that had gone missing"
    );
}

/// Round seven's third point: the mana pips are walked by the two horizontal
/// keys, and either of them reaches the strip from a written row at once.
///
/// Reported as *"bei den Mana Symbolen, die kann man mit A und D navigieren.
/// Wenn man A oder D navigiert, switcht es sofort zur Mana Zeile, mit W und S
/// wie gehabt hoch und runter."* The sheet is the only one in the client with
/// two directions on it — a centred row of pips above a column of sentences —
/// and all four keys used to step the flat list by one, so `D` on a pip moved
/// sideways and `D` on a sentence moved down.
#[test]
fn the_horizontal_keys_walk_the_pips_and_jump_to_them() {
    use baylee_client::input::{ability_menu_keys, activate_card};
    use baylee_client::keys::Fired;
    use baylee_client::{Duel, abilities};
    use baylee_client_core::interaction::Interaction;
    use baylee_client_core::prefs::Action;

    let mut preset = bubble_preset();
    preset.seats[0].starting_battlefield = vec![entry(SQUAD), entry(HARABAZ)];
    let mut table = Table::open_with(&preset);
    table.walk_to_main();

    let druid = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name == "Harabaz Druid")
        .expect("the creature starts on the table")
        .id;

    let mut duel = Duel::default();
    duel.view = Some(table.view().clone());
    duel.interaction = Some(Interaction::new(
        table.pending.clone().expect("priority"),
        PlayerId::new(0),
    ));
    baylee_client::rebuild_board(&mut duel);

    activate_card(&mut duel, druid);
    assert_eq!(duel.ability_menu, Some(druid), "the sheet opens");
    let options = abilities::options(
        baylee_client_core::Lang::En,
        duel.view.as_ref().expect("a view"),
        duel.interaction.as_ref().expect("priority"),
        druid,
    );
    let split = abilities::Split::of(&options);
    assert_eq!((split.pips, split.rows), (5, 1), "five pips and a sentence");

    let key = |duel: &mut Duel, action| ability_menu_keys(Fired::of_actions(&[action]), duel);

    // Down walks everything there is, header included, exactly as before.
    assert!(key(&mut duel, Action::CursorDown));
    assert_eq!(duel.ability_pick, 1, "the second pip");

    // Right walks the header and wraps inside it, without ever reaching the
    // sentence: five pips, not six options.
    for expected in [2, 3, 4, 0] {
        assert!(key(&mut duel, Action::CursorRight));
        assert_eq!(duel.ability_pick, expected, "the header is its own ring");
    }
    assert!(key(&mut duel, Action::CursorLeft));
    assert_eq!(duel.ability_pick, 4, "and the other way round it");

    // Onto the written row, by the vertical key that owns the column.
    assert!(key(&mut duel, Action::CursorDown));
    assert_eq!(duel.ability_pick, split.option(0), "the sentence");

    // And one horizontal press arrives on the strip, at the end it was
    // heading for — this is the half the owner asked for by name.
    assert!(key(&mut duel, Action::CursorRight));
    assert_eq!(duel.ability_pick, 0, "rightwards enters at the first pip");
    assert!(
        key(&mut duel, Action::CursorUp),
        "up off the first pip is the top of the whole column"
    );
    assert_eq!(
        duel.ability_pick,
        split.option(0),
        "which wraps round to the sentence"
    );
    assert!(key(&mut duel, Action::CursorLeft));
    assert_eq!(duel.ability_pick, 4, "leftwards enters at the last");
}
