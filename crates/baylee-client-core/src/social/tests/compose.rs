//! The composer: Enter, Esc, history and Tab precedence, card links, spans in characters, limits and drafts (§10.4, §12.1, §11.9).

use baylee_core::ids::CardIndex;
use baylee_view::SeenIn;

use crate::bugreport::refs::{Candidates, CardRef, PlayerRef, RefPrint};
use crate::social::commands::{GroupRef, Target};
use crate::social::compose::{
    ComposeContext, Composer, ComposerKey, ComposerOutcome, LinkedCard, SENT_KEPT, SuggestItem,
};
use crate::social::convs::{COMPOSE_LIMIT, ConvId};
use crate::social::text::MAX_LINE_BREAKS;
use crate::social::words::ChatPhrase;

fn cx() -> ComposeContext {
    ComposeContext::default()
}

fn card_ref(name: &str, index: u32) -> CardRef {
    CardRef {
        name: name.to_string(),
        english: name.to_string(),
        card: CardIndex::new(index),
        face: 0,
        art: None,
        print: Some(RefPrint {
            scryfall_id: format!("print-{index}"),
            lang: "en".to_string(),
            finish: "nonfoil".to_string(),
        }),
        object: None,
        zone: None,
        owner: None,
        kind: None,
    }
}

fn player_ref(name: &str, tag: Option<&str>) -> PlayerRef {
    PlayerRef {
        name: name.to_string(),
        seat: None,
        tag: tag.map(str::to_string),
    }
}

/// A composer that knows two cards and two players.
fn composer() -> Composer {
    let mut c = Composer::new();
    c.set_candidates(Candidates {
        cards: vec![card_ref("Lightning Bolt", 7), card_ref("Llanowar Elves", 8)],
        players: vec![
            player_ref("Bob#0012", Some("0012")),
            player_ref("Carl", None),
        ],
    });
    c
}

fn key(c: &mut Composer, k: ComposerKey) -> ComposerOutcome {
    c.key(k, &cx())
}

/// Types `text` and sends it, as the shell does.
fn send(c: &mut Composer, text: &str) {
    c.typed(text);
    c.sent();
}

fn linked(seen: Option<SeenIn>, face_down: bool) -> LinkedCard {
    LinkedCard {
        card: CardIndex::new(7),
        face: 0,
        print: None,
        seen,
        face_down,
        name: "Lightning Bolt".to_string(),
        english: "Lightning Bolt".to_string(),
    }
}

fn conv(game: &str) -> ConvId {
    ConvId::Table { game: game.into() }
}

// --- Enter ---------------------------------------------------------------

#[test]
fn enter_with_the_popover_up_takes_the_suggestion() {
    let mut c = composer();
    c.typed("cast #lig");
    let suggest = c.suggestions().expect("a popover");
    assert_eq!(suggest.items, [SuggestItem::Card(0)]);
    assert_eq!(
        c.suggestion_label(&suggest.items[0]),
        Some("Lightning Bolt")
    );
    assert_eq!(key(&mut c, ComposerKey::Enter), ComposerOutcome::Changed);
    assert_eq!(c.text(), "cast [Lightning Bolt] ");
    assert!(c.suggestions().is_none());
    assert_eq!(
        c.buffer().cursor(),
        c.text().len(),
        "the caret follows the name"
    );
}

#[test]
fn enter_without_the_popover_submits() {
    let mut c = composer();
    c.typed("good luck");
    assert!(c.suggestions().is_none());
    assert_eq!(key(&mut c, ComposerKey::Enter), ComposerOutcome::Submit);
    assert_eq!(
        c.text(),
        "good luck",
        "the shell reads it, then calls sent()"
    );
}

#[test]
fn enter_after_taking_a_suggestion_submits_a_body_with_the_link() {
    let mut c = composer();
    c.typed("#lig");
    key(&mut c, ComposerKey::Enter);
    assert_eq!(key(&mut c, ComposerKey::Enter), ComposerOutcome::Submit);
    let (target, body) = c.body().expect("a body");
    assert_eq!(target, Target::Sticky);
    assert_eq!(body.text, "[Lightning Bolt]");
    assert_eq!(body.links.len(), 1);
    assert_eq!(body.links[0].card, 7);
}

#[test]
fn a_hash_with_no_candidates_raises_no_popover_and_enter_submits() {
    let mut c = Composer::new();
    c.typed("see #lig");
    assert!(c.suggestions().is_none());
    assert_eq!(key(&mut c, ComposerKey::Enter), ComposerOutcome::Submit);
}

#[test]
fn enter_on_an_empty_box_closes_it() {
    let mut c = composer();
    assert_eq!(key(&mut c, ComposerKey::Enter), ComposerOutcome::Close);
}

#[test]
fn enter_on_a_box_of_only_whitespace_closes_it_and_empties_it() {
    let mut c = composer();
    c.typed("   ");
    assert_eq!(key(&mut c, ComposerKey::Enter), ComposerOutcome::Close);
    assert_eq!(c.text(), "");
}

#[test]
fn up_and_down_move_the_highlight_in_the_popover_and_enter_takes_it() {
    let mut c = composer();
    c.typed("#l");
    assert_eq!(c.suggestions().unwrap().items.len(), 2);
    assert_eq!(c.suggestions().unwrap().highlight, 0);
    assert_eq!(key(&mut c, ComposerKey::Down), ComposerOutcome::Changed);
    assert_eq!(c.suggestions().unwrap().highlight, 1);
    assert_eq!(key(&mut c, ComposerKey::Down), ComposerOutcome::Changed);
    assert_eq!(c.suggestions().unwrap().highlight, 0, "wraps round");
    assert_eq!(key(&mut c, ComposerKey::Up), ComposerOutcome::Changed);
    assert_eq!(c.suggestions().unwrap().highlight, 1, "wraps back");
    key(&mut c, ComposerKey::Enter);
    assert_eq!(c.text(), "[Llanowar Elves] ");
}

#[test]
fn at_offers_players_and_taking_one_writes_a_bracketed_mention() {
    let mut c = composer();
    c.typed("hey @bo");
    assert_eq!(c.suggestions().unwrap().items, [SuggestItem::Player(0)]);
    key(&mut c, ComposerKey::Enter);
    assert_eq!(c.text(), "hey [@Bob#0012] ");
    let (_, body) = c.body().unwrap();
    assert_eq!(body.mentions.len(), 1);
    assert_eq!(body.mentions[0].tag, "0012");
}

// --- Esc -----------------------------------------------------------------

#[test]
fn esc_closes_the_popover_first_and_the_box_second() {
    let mut c = composer();
    c.typed("#lig");
    assert!(c.suggestions().is_some());
    assert_eq!(key(&mut c, ComposerKey::Esc), ComposerOutcome::Changed);
    assert!(c.suggestions().is_none());
    assert_eq!(c.text(), "#lig", "the text stays");
    assert_eq!(key(&mut c, ComposerKey::Esc), ComposerOutcome::Close);
    assert_eq!(c.text(), "#lig", "closing keeps the draft in the box");
}

#[test]
fn esc_without_a_popover_closes_the_box() {
    let mut c = composer();
    assert_eq!(key(&mut c, ComposerKey::Esc), ComposerOutcome::Close);
    c.typed("hello");
    assert_eq!(key(&mut c, ComposerKey::Esc), ComposerOutcome::Close);
}

// --- Up and Down through what was sent ------------------------------------

#[test]
fn up_and_down_walk_the_sent_lines_and_stop_at_the_ends() {
    let mut c = composer();
    for line in ["one", "two", "three"] {
        send(&mut c, line);
    }
    assert_eq!(c.text(), "", "sent() empties the box");
    c.typed("draft");

    assert_eq!(key(&mut c, ComposerKey::Up), ComposerOutcome::Changed);
    assert_eq!(c.text(), "three");
    assert_eq!(key(&mut c, ComposerKey::Up), ComposerOutcome::Changed);
    assert_eq!(c.text(), "two");
    assert_eq!(key(&mut c, ComposerKey::Up), ComposerOutcome::Changed);
    assert_eq!(c.text(), "one");
    assert_eq!(
        key(&mut c, ComposerKey::Up),
        ComposerOutcome::Nothing,
        "the oldest"
    );
    assert_eq!(c.text(), "one");

    assert_eq!(key(&mut c, ComposerKey::Down), ComposerOutcome::Changed);
    assert_eq!(c.text(), "two");
    assert_eq!(key(&mut c, ComposerKey::Down), ComposerOutcome::Changed);
    assert_eq!(c.text(), "three");
    // Past the newest, what was typed comes back.
    assert_eq!(key(&mut c, ComposerKey::Down), ComposerOutcome::Changed);
    assert_eq!(c.text(), "draft");
    assert_eq!(
        key(&mut c, ComposerKey::Down),
        ComposerOutcome::Nothing,
        "the newest"
    );
    assert_eq!(c.text(), "draft");
}

#[test]
fn up_with_nothing_sent_does_nothing() {
    let mut c = composer();
    c.typed("draft");
    assert_eq!(key(&mut c, ComposerKey::Up), ComposerOutcome::Nothing);
    assert_eq!(key(&mut c, ComposerKey::Down), ComposerOutcome::Nothing);
    assert_eq!(c.text(), "draft");
}

#[test]
fn only_the_last_thirty_two_sent_lines_are_kept() {
    assert_eq!(SENT_KEPT, 32);
    let mut c = composer();
    for i in 0..40 {
        send(&mut c, &format!("line {i}"));
    }
    for step in 0..SENT_KEPT {
        assert_eq!(
            key(&mut c, ComposerKey::Up),
            ComposerOutcome::Changed,
            "step {step}"
        );
    }
    assert_eq!(
        c.text(),
        "line 8",
        "40 sent, 32 kept: the oldest reachable is line 8"
    );
    assert_eq!(key(&mut c, ComposerKey::Up), ComposerOutcome::Nothing);
    assert_eq!(c.text(), "line 8");
}

#[test]
fn a_line_sent_twice_in_a_row_is_remembered_once() {
    let mut c = composer();
    send(&mut c, "gg");
    send(&mut c, "gg");
    assert_eq!(key(&mut c, ComposerKey::Up), ComposerOutcome::Changed);
    assert_eq!(key(&mut c, ComposerKey::Up), ComposerOutcome::Nothing);
}

#[test]
fn an_empty_line_is_not_remembered() {
    let mut c = composer();
    send(&mut c, "   ");
    assert_eq!(key(&mut c, ComposerKey::Up), ComposerOutcome::Nothing);
}

#[test]
fn a_sent_line_is_remembered_trimmed() {
    let mut c = composer();
    send(&mut c, "  hello  ");
    key(&mut c, ComposerKey::Up);
    assert_eq!(c.text(), "hello");
}

#[test]
fn typing_after_recalling_starts_the_walk_again_from_the_newest() {
    let mut c = composer();
    send(&mut c, "one");
    send(&mut c, "two");
    key(&mut c, ComposerKey::Up);
    key(&mut c, ComposerKey::Up);
    assert_eq!(c.text(), "one");
    c.typed("!");
    assert_eq!(c.text(), "one!");
    key(&mut c, ComposerKey::Up);
    assert_eq!(c.text(), "two");
    key(&mut c, ComposerKey::Down);
    assert_eq!(c.text(), "one!", "what was typed comes back");
}

#[test]
fn up_in_a_box_of_several_lines_moves_between_lines_before_walking_the_sent_ones() {
    let mut c = composer();
    send(&mut c, "old");
    c.typed("ab");
    key(&mut c, ComposerKey::ShiftEnter);
    c.typed("cd");
    assert_eq!(c.text(), "ab\ncd");
    assert_eq!(key(&mut c, ComposerKey::Up), ComposerOutcome::Changed);
    assert_eq!(c.text(), "ab\ncd", "moved the caret only");
    assert!(c.buffer().cursor() <= 2, "now on the first line");
    assert_eq!(key(&mut c, ComposerKey::Up), ComposerOutcome::Changed);
    assert_eq!(c.text(), "old");
}

// --- Tab -------------------------------------------------------------------

fn targets() -> ComposeContext {
    ComposeContext {
        targets: vec![
            Target::Table,
            Target::Team,
            Target::Whisper("Bob#0012".into()),
            Target::Group(GroupRef::Number(1)),
        ],
        names: vec![
            "Bob#0012".to_string(),
            "Bobby#0099".to_string(),
            "Alice#0001".to_string(),
        ],
    }
}

#[test]
fn tab_takes_the_suggestion_before_anything_else() {
    let mut c = composer();
    c.set_target(Target::Table);
    c.typed("/w Bo #lig");
    assert!(c.suggestions().is_some());
    assert_eq!(
        c.key(ComposerKey::Tab, &targets()),
        ComposerOutcome::Changed
    );
    assert_eq!(c.text(), "/w Bo [Lightning Bolt] ");
    assert_eq!(*c.target(), Target::Table, "the target did not cycle");
}

#[test]
fn shift_tab_takes_the_suggestion_too() {
    let mut c = composer();
    c.typed("#lig");
    assert_eq!(
        c.key(ComposerKey::ShiftTab, &targets()),
        ComposerOutcome::Changed
    );
    assert_eq!(c.text(), "[Lightning Bolt] ");
    assert_eq!(*c.target(), Target::Sticky);
}

#[test]
fn tab_in_a_whisper_name_completes_it_before_the_target_cycles() {
    let mut c = composer();
    c.set_target(Target::Table);
    c.typed("/w Bo");
    assert_eq!(
        c.key(ComposerKey::Tab, &targets()),
        ComposerOutcome::Changed
    );
    assert_eq!(c.text(), "/w Bob#0012");
    assert_eq!(*c.target(), Target::Table);
}

#[test]
fn repeated_tab_cycles_through_the_names_that_match() {
    let mut c = composer();
    c.typed("/w bo");
    let ctx = targets();
    c.key(ComposerKey::Tab, &ctx);
    assert_eq!(c.text(), "/w Bob#0012");
    c.key(ComposerKey::Tab, &ctx);
    assert_eq!(c.text(), "/w Bobby#0099");
    c.key(ComposerKey::Tab, &ctx);
    assert_eq!(c.text(), "/w Bob#0012", "wraps round");
    assert_eq!(*c.target(), Target::Sticky);
}

#[test]
fn tab_after_a_space_completes_the_first_name() {
    let mut c = composer();
    c.typed("/w ");
    c.key(ComposerKey::Tab, &targets());
    assert_eq!(c.text(), "/w Bob#0012");
}

#[test]
fn name_completion_works_in_the_other_commands_that_take_a_name() {
    for command in [
        "tell", "msg", "invite", "watch", "friend", "block", "ignore", "nick",
    ] {
        let mut c = composer();
        c.typed(&format!("/{command} ali"));
        c.key(ComposerKey::Tab, &targets());
        assert_eq!(c.text(), format!("/{command} Alice#0001"), "/{command}");
    }
}

#[test]
fn a_typed_key_between_tabs_restarts_the_name_cycle() {
    let mut c = composer();
    c.typed("/w Bo");
    let ctx = targets();
    c.key(ComposerKey::Tab, &ctx);
    assert_eq!(c.text(), "/w Bob#0012");
    c.key(ComposerKey::End, &ctx);
    c.key(ComposerKey::Tab, &ctx);
    assert_eq!(
        c.text(),
        "/w Bob#0012",
        "Bob#0012 is now the prefix, so it is the only match"
    );
}

#[test]
fn tab_cycles_the_target_when_no_name_is_being_typed() {
    let ctx = targets();
    for text in ["hello", "/t Bo", "/w Bob#0012 and then some", "/w Zed", ""] {
        let mut c = composer();
        c.typed(text);
        assert_eq!(
            c.key(ComposerKey::Tab, &ctx),
            ComposerOutcome::Changed,
            "{text:?}"
        );
        assert_eq!(*c.target(), Target::Table, "{text:?}: the first stop");
        assert_eq!(c.text(), text, "the text is not touched");
    }
}

#[test]
fn tab_cycles_the_targets_in_order_and_wraps() {
    let ctx = targets();
    let mut c = composer();
    let mut seen = Vec::new();
    for _ in 0..5 {
        c.key(ComposerKey::Tab, &ctx);
        seen.push(c.target().clone());
    }
    assert_eq!(
        seen,
        [
            Target::Table,
            Target::Team,
            Target::Whisper("Bob#0012".into()),
            Target::Group(GroupRef::Number(1)),
            Target::Table,
        ]
    );
}

#[test]
fn shift_tab_cycles_the_targets_backwards() {
    let ctx = targets();
    let mut c = composer();
    c.set_target(Target::Team);
    c.key(ComposerKey::ShiftTab, &ctx);
    assert_eq!(*c.target(), Target::Table);
    c.key(ComposerKey::ShiftTab, &ctx);
    assert_eq!(
        *c.target(),
        Target::Group(GroupRef::Number(1)),
        "wraps to the last"
    );
    c.key(ComposerKey::ShiftTab, &ctx);
    assert_eq!(*c.target(), Target::Whisper("Bob#0012".into()));
}

#[test]
fn shift_tab_from_the_sticky_target_goes_to_the_last_one() {
    let mut c = composer();
    c.key(ComposerKey::ShiftTab, &targets());
    assert_eq!(*c.target(), Target::Group(GroupRef::Number(1)));
}

#[test]
fn shift_tab_in_a_whisper_name_cycles_the_target_not_the_name() {
    let mut c = composer();
    c.typed("/w Bo");
    c.key(ComposerKey::ShiftTab, &targets());
    assert_eq!(c.text(), "/w Bo");
    assert_eq!(*c.target(), Target::Group(GroupRef::Number(1)));
}

#[test]
fn tab_with_no_targets_does_nothing() {
    let mut c = composer();
    c.typed("hello");
    assert_eq!(key(&mut c, ComposerKey::Tab), ComposerOutcome::Nothing);
    assert_eq!(key(&mut c, ComposerKey::ShiftTab), ComposerOutcome::Nothing);
    assert_eq!(*c.target(), Target::Sticky);
}

// --- Right-clicking a card -------------------------------------------------

#[test]
fn a_teammates_shared_hand_a_controlled_hand_and_a_face_down_card_are_refused() {
    for (what, card) in [
        ("shared hand", linked(Some(SeenIn::SharedHand), false)),
        (
            "controlled hand",
            linked(Some(SeenIn::ControlledHand), false),
        ),
        (
            "face down on the battlefield",
            linked(Some(SeenIn::Battlefield), true),
        ),
        ("face down in the deck builder", linked(None, true)),
        ("face down in the hand", linked(Some(SeenIn::Hand), true)),
    ] {
        let mut c = composer();
        assert_eq!(c.insert_card(card), Some(ChatPhrase::PrivateCard), "{what}");
        assert_eq!(c.text(), "", "{what}: nothing is inserted");
        assert!(c.body().is_none(), "{what}");
    }
}

#[test]
fn a_refused_card_leaves_no_link_behind() {
    let mut c = Composer::new();
    c.insert_card(linked(Some(SeenIn::SharedHand), false));
    c.typed("[Lightning Bolt]");
    let (_, body) = c.body().unwrap();
    assert!(
        body.links.is_empty(),
        "no candidate, no pick: it stays text"
    );
}

#[test]
fn descriptions_of_an_object_as_it_was_are_refused_as_well() {
    for seen in [
        SeenIn::TargetingSource,
        SeenIn::DamageSource,
        SeenIn::TargetObject,
    ] {
        let mut c = composer();
        assert_eq!(
            c.insert_card(linked(Some(seen), false)),
            Some(ChatPhrase::PrivateCard),
            "{seen:?}"
        );
        assert_eq!(c.text(), "");
    }
}

#[test]
fn a_battlefield_card_is_linked_without_a_word() {
    let mut c = Composer::new();
    assert_eq!(
        c.insert_card(linked(Some(SeenIn::Battlefield), false)),
        None
    );
    assert_eq!(c.text(), "[Lightning Bolt] ");
    let (_, body) = c.body().unwrap();
    assert_eq!(body.links.len(), 1);
    assert_eq!(body.links[0].at, [0, 16]);
    assert_eq!(body.links[0].card, 7);
}

#[test]
fn every_public_zone_and_the_deck_builder_link_silently() {
    for seen in [
        None,
        Some(SeenIn::Battlefield),
        Some(SeenIn::Stack),
        Some(SeenIn::Graveyard),
        Some(SeenIn::Exile),
        Some(SeenIn::Command),
        Some(SeenIn::Commander),
        Some(SeenIn::LibraryTop),
    ] {
        let mut c = Composer::new();
        assert_eq!(c.insert_card(linked(seen, false)), None, "{seen:?}");
        assert_eq!(c.text(), "[Lightning Bolt] ", "{seen:?}");
    }
}

#[test]
fn a_card_from_the_own_hand_is_linked_with_a_hint_only_the_first_time() {
    let mut c = Composer::new();
    assert_eq!(
        c.insert_card(linked(Some(SeenIn::Hand), false)),
        Some(ChatPhrase::OwnCardHint)
    );
    assert_eq!(c.text(), "[Lightning Bolt] ", "it is linked all the same");
    assert_eq!(c.insert_card(linked(Some(SeenIn::Hand), false)), None);
    assert_eq!(c.text(), "[Lightning Bolt] [Lightning Bolt] ");
    // The one hint serves both kinds of private card.
    assert_eq!(c.insert_card(linked(Some(SeenIn::LookingAt), false)), None);
}

#[test]
fn a_card_being_looked_at_hints_too() {
    let mut c = Composer::new();
    assert_eq!(
        c.insert_card(linked(Some(SeenIn::LookingAt), false)),
        Some(ChatPhrase::OwnCardHint)
    );
    assert_eq!(c.text(), "[Lightning Bolt] ");
}

#[test]
fn the_hint_is_not_spent_by_a_public_card() {
    let mut c = Composer::new();
    assert_eq!(c.insert_card(linked(Some(SeenIn::Stack), false)), None);
    assert_eq!(
        c.insert_card(linked(Some(SeenIn::Hand), false)),
        Some(ChatPhrase::OwnCardHint)
    );
}

#[test]
fn a_card_is_linked_at_the_caret() {
    let mut c = Composer::new();
    c.typed("ab");
    key(&mut c, ComposerKey::Left);
    c.insert_card(linked(Some(SeenIn::Battlefield), false));
    assert_eq!(c.text(), "a[Lightning Bolt] b");
    let (_, body) = c.body().unwrap();
    assert_eq!(body.links[0].at, [1, 17]);
}

#[test]
fn a_card_that_would_pass_the_limit_is_not_inserted() {
    let mut c = Composer::new();
    key(&mut c, ComposerKey::Paste("a".repeat(COMPOSE_LIMIT - 10)));
    let before = c.text().to_string();
    assert_eq!(
        c.insert_card(linked(Some(SeenIn::Battlefield), false)),
        None
    );
    assert_eq!(c.text(), before);
}

#[test]
fn a_link_whose_brackets_were_edited_away_is_forgotten() {
    let mut c = Composer::new();
    c.insert_card(linked(Some(SeenIn::Battlefield), false));
    key(&mut c, ComposerKey::Backspace); // the space
    assert_eq!(c.body().unwrap().1.links.len(), 1);
    key(&mut c, ComposerKey::Backspace); // the closing bracket
    c.typed("]");
    assert!(
        c.body().unwrap().1.links.is_empty(),
        "the pick went with its bracket; a typed bracket is only text"
    );
}

// --- Spans in characters ----------------------------------------------------

#[test]
fn links_and_mentions_are_counted_in_characters_not_bytes() {
    let c = composer();
    let text = "Jörg 🙂 [Lightning Bolt] [@Bob#0012]";
    assert!(
        text.len() == text.chars().count() + 4,
        "bytes and characters differ"
    );
    let body = c.body_of(text, false);
    assert_eq!(body.text, text);
    assert_eq!(body.links.len(), 1);
    assert_eq!(body.links[0].at, [7, 23]);
    assert_eq!(body.links[0].card, 7);
    assert_eq!(body.span_text(body.links[0].at), "[Lightning Bolt]");
    assert_eq!(body.mentions.len(), 1);
    assert_eq!(body.mentions[0].at, [24, 35]);
    assert_eq!(body.mentions[0].tag, "0012");
    assert_eq!(body.span_text(body.mentions[0].at), "[@Bob#0012]");
}

#[test]
fn a_link_carries_its_face_and_printing() {
    let c = composer();
    let body = c.body_of("[Lightning Bolt]", false);
    let print = body.links[0].print.as_ref().expect("a printing");
    assert_eq!(print.scryfall_id, "print-7");
    assert_eq!(print.lang, "en");
    assert_eq!(body.links[0].face, 0);
}

#[test]
fn a_card_is_found_by_its_english_name_and_without_regard_to_case() {
    let mut c = Composer::new();
    let mut bolt = card_ref("Blitz", 7);
    bolt.english = "Lightning Bolt".to_string();
    c.set_candidates(Candidates {
        cards: vec![bolt],
        players: Vec::new(),
    });
    assert_eq!(
        c.body_of("[lightning bolt] and [BLITZ]", false).links.len(),
        2
    );
}

#[test]
fn the_same_card_twice_is_two_links() {
    let c = composer();
    let body = c.body_of("[Lightning Bolt][Lightning Bolt]", false);
    let spans: Vec<[usize; 2]> = body.links.iter().map(|l| l.at).collect();
    assert_eq!(spans, [[0, 16], [16, 32]]);
}

#[test]
fn a_player_without_a_tag_makes_no_mention() {
    let c = composer();
    let body = c.body_of("hello [@Carl]", false);
    assert!(body.mentions.is_empty());
    assert!(body.links.is_empty());
    assert_eq!(body.text, "hello [@Carl]");
}

#[test]
fn a_bracket_that_names_nothing_stays_text() {
    let c = composer();
    let body = c.body_of("[Black Lotus] [@Nobody#0001] [] [@]", false);
    assert!(body.links.is_empty());
    assert!(body.mentions.is_empty());
}

#[test]
fn body_of_keeps_the_emote_flag() {
    let c = composer();
    assert!(c.body_of("waves", true).emote);
    assert!(!c.body_of("waves", false).emote);
}

#[test]
fn body_reads_the_trimmed_text_with_the_chips_target() {
    let mut c = composer();
    c.set_target(Target::Table);
    c.typed("  [Lightning Bolt]  ");
    let (target, body) = c.body().expect("a body");
    assert_eq!(target, Target::Table);
    assert_eq!(body.text, "[Lightning Bolt]");
    assert_eq!(body.links[0].at, [0, 16], "counted from the trimmed text");
}

#[test]
fn body_is_none_for_an_empty_or_blank_box() {
    let mut c = composer();
    assert!(c.body().is_none());
    c.typed("  ");
    assert!(c.body().is_none());
}

// --- Line breaks and limits ---------------------------------------------------

#[test]
fn shift_enter_adds_at_most_six_line_breaks() {
    assert_eq!(MAX_LINE_BREAKS, 6);
    let mut c = composer();
    c.typed("x");
    for n in 1..=8 {
        let outcome = key(&mut c, ComposerKey::ShiftEnter);
        if n <= 6 {
            assert_eq!(outcome, ComposerOutcome::Changed, "break {n}");
        } else {
            assert_eq!(outcome, ComposerOutcome::Nothing, "break {n}");
        }
    }
    assert_eq!(c.text().matches('\n').count(), 6);
}

#[test]
fn shift_enter_at_the_character_limit_adds_nothing() {
    let mut c = composer();
    key(&mut c, ComposerKey::Paste("a".repeat(COMPOSE_LIMIT)));
    assert_eq!(
        key(&mut c, ComposerKey::ShiftEnter),
        ComposerOutcome::Nothing
    );
    assert_eq!(c.chars(), COMPOSE_LIMIT);
}

#[test]
fn paste_stops_at_the_limit() {
    assert_eq!(COMPOSE_LIMIT, 1000);
    let mut c = composer();
    assert_eq!(
        key(&mut c, ComposerKey::Paste("ä".repeat(1500))),
        ComposerOutcome::Changed
    );
    assert_eq!(c.chars(), 1000);
}

#[test]
fn paste_fills_only_the_room_that_is_left() {
    let mut c = composer();
    c.typed(&"a".repeat(990));
    key(&mut c, ComposerKey::Paste("b".repeat(50)));
    assert_eq!(c.chars(), 1000);
    assert!(c.text().ends_with(&"b".repeat(10)));
    assert!(!c.text().ends_with(&"b".repeat(11)));
}

#[test]
fn paste_drops_control_and_hidden_characters_and_carriage_returns() {
    let mut c = composer();
    key(
        &mut c,
        ComposerKey::Paste("a\u{7}b\tc\r\nd\u{202E}e\u{200B}f\u{FEFF}g".to_string()),
    );
    assert_eq!(c.text(), "abc\ndefg");
}

#[test]
fn paste_keeps_six_line_breaks_and_turns_the_rest_into_spaces() {
    let mut c = composer();
    key(&mut c, ComposerKey::Paste("l\n".repeat(9)));
    assert_eq!(c.text().matches('\n').count(), 6);
    assert_eq!(c.text(), "l\nl\nl\nl\nl\nl\nl l l ");
}

#[test]
fn paste_counts_the_line_breaks_already_in_the_box() {
    let mut c = composer();
    c.typed("x");
    for _ in 0..4 {
        key(&mut c, ComposerKey::ShiftEnter);
    }
    key(&mut c, ComposerKey::Paste("\n\n\n\n".to_string()));
    assert_eq!(c.text().matches('\n').count(), 6);
    assert!(c.text().ends_with("\n\n  "));
}

#[test]
fn typed_text_stops_at_the_limit_and_drops_control_characters() {
    let mut c = composer();
    c.typed("a\u{7}b\nc\td");
    assert_eq!(c.text(), "abcd");
    c.typed(&"z".repeat(2000));
    assert_eq!(c.chars(), 1000);
    c.typed("more");
    assert_eq!(c.chars(), 1000);
}

#[test]
fn typed_text_drops_hidden_characters_as_paste_does() {
    let mut c = composer();
    c.typed("a\u{202E}b\u{200B}c\u{FEFF}d\u{2066}e\u{200F}f\u{061C}g");
    assert_eq!(c.text(), "abcdefg");
    let mut p = composer();
    key(
        &mut p,
        ComposerKey::Paste("a\u{202E}b\u{200B}c\u{FEFF}d\u{2066}e\u{200F}f\u{061C}g".into()),
    );
    assert_eq!(p.text(), c.text());
}

#[test]
fn typed_hidden_characters_do_not_shift_a_links_span() {
    let mut c = composer();
    c.typed("hi\u{200B} [Lightning\u{202E} Bolt]");
    let (_, body) = c.body().expect("a body");
    assert_eq!(body.text, "hi [Lightning Bolt]");
    assert_eq!(body.links.len(), 1);
    assert_eq!(body.links[0].at, [3, 19]);
    // The receiver finds nothing to clean, so it keeps the link.
    let checked = body.checked().expect("a body");
    assert_eq!(checked.links.len(), 1);
}

#[test]
fn typed_hidden_characters_do_not_count_against_the_limit() {
    let mut c = composer();
    c.typed(&format!("{}{}", "\u{200B}".repeat(500), "a".repeat(1000)));
    assert_eq!(c.chars(), 1000);
}

#[test]
fn paste_replaces_the_selection_and_select_all_copies_everything() {
    let mut c = composer();
    c.typed("hello world");
    key(&mut c, ComposerKey::SelectAll);
    assert_eq!(
        key(&mut c, ComposerKey::Copy),
        ComposerOutcome::Copy("hello world".into())
    );
    key(&mut c, ComposerKey::Paste("bye".into()));
    assert_eq!(c.text(), "bye");
}

#[test]
fn cut_copies_the_selection_and_deletes_it() {
    let mut c = composer();
    c.typed("hello");
    key(&mut c, ComposerKey::SelectAll);
    assert_eq!(
        key(&mut c, ComposerKey::Cut),
        ComposerOutcome::Copy("hello".into())
    );
    assert_eq!(c.text(), "");
    assert_eq!(key(&mut c, ComposerKey::Cut), ComposerOutcome::Nothing);
    assert_eq!(key(&mut c, ComposerKey::Copy), ComposerOutcome::Nothing);
}

#[test]
fn the_shell_keys_pass_through_as_outcomes() {
    let mut c = composer();
    assert_eq!(
        key(&mut c, ComposerKey::PageUp),
        ComposerOutcome::ScrollHistory(-1)
    );
    assert_eq!(
        key(&mut c, ComposerKey::PageDown),
        ComposerOutcome::ScrollHistory(1)
    );
    assert_eq!(
        key(&mut c, ComposerKey::AltDigit(3)),
        ComposerOutcome::SwitchTab(3)
    );
}

#[test]
fn backspace_on_an_empty_box_does_nothing() {
    let mut c = composer();
    assert_eq!(
        key(&mut c, ComposerKey::Backspace),
        ComposerOutcome::Nothing
    );
    c.typed("ab");
    assert_eq!(
        key(&mut c, ComposerKey::Backspace),
        ComposerOutcome::Changed
    );
    assert_eq!(c.text(), "a");
}

// --- Opening and drafts ---------------------------------------------------------

#[test]
fn open_with_a_prefix_types_it_and_an_empty_prefix_keeps_the_draft() {
    let mut c = composer();
    c.open_with("/r Bob#0012 ");
    assert_eq!(c.text(), "/r Bob#0012 ");
    assert_eq!(c.buffer().cursor(), c.text().len());
    c.open_with("");
    assert_eq!(c.text(), "/r Bob#0012 ");
    c.open_with("/");
    assert_eq!(c.text(), "/");
}

#[test]
fn a_draft_is_kept_per_conversation_and_restored_once() {
    let mut c = composer();
    let (a, b) = (conv("g1"), conv("g2"));
    c.typed("half a thought");
    c.stash_draft(&a);
    assert_eq!(c.text(), "", "stashing empties the box");

    c.typed("another one");
    c.stash_draft(&b);

    c.restore_draft(&a);
    assert_eq!(c.text(), "half a thought");
    assert_eq!(c.buffer().cursor(), c.text().len());
    c.stash_draft(&conv("elsewhere"));
    c.restore_draft(&a);
    assert_eq!(c.text(), "", "a draft is restored once");

    c.restore_draft(&b);
    assert_eq!(c.text(), "another one");
}

#[test]
fn stashing_an_empty_box_forgets_the_conversations_old_draft() {
    let mut c = composer();
    let a = conv("g1");
    c.typed("draft");
    c.stash_draft(&a);
    c.stash_draft(&a); // the box is empty now
    c.restore_draft(&a);
    assert_eq!(c.text(), "");
}

#[test]
fn restoring_a_conversation_without_a_draft_leaves_the_box_alone() {
    let mut c = composer();
    c.typed("typing");
    c.restore_draft(&conv("none"));
    assert_eq!(c.text(), "typing");
}

#[test]
fn a_restored_draft_raises_the_popover_it_ended_in() {
    let mut c = composer();
    let a = conv("g1");
    c.typed("cast #lig");
    c.stash_draft(&a);
    assert!(c.suggestions().is_none(), "stashing puts the popover away");
    c.restore_draft(&a);
    assert!(c.suggestions().is_some());
}

#[test]
fn sent_empties_the_box_and_forgets_the_picks_and_popover() {
    let mut c = composer();
    c.insert_card(linked(Some(SeenIn::Battlefield), false));
    c.typed("#lig");
    c.sent();
    assert_eq!(c.text(), "");
    assert!(c.suggestions().is_none());
    c.typed("[Lightning Bolt]");
    // The candidate list still knows the card, so this one is a link again;
    // what is gone is the pick itself.
    assert_eq!(c.body().unwrap().1.links.len(), 1);
}

#[test]
fn clear_empties_the_box_without_remembering_the_line() {
    let mut c = composer();
    c.typed("/who");
    c.clear();
    assert_eq!(c.text(), "");
    assert_eq!(key(&mut c, ComposerKey::Up), ComposerOutcome::Nothing);
}
