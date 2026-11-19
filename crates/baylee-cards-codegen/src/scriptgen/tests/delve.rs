//! Unit tests for reading delve (`K:Delve`, CR 702.66a), a count of the
//! cards exiled with the permanent its spell became (CR 607.2q), and
//! "whenever a [filter] card leaves your graveyard", on made-up scripts in
//! Murktide Regent's shape.

use super::*;

const COUNT: &str = "K:etbCounter:P1P1:X:no Condition:CARDNAME enters with a +1/+1 counter on it for each instant and sorcery card exiled with it.\n\
     SVar:X:Count$ValidExile Instant.ExiledWithSource,Sorcery.ExiledWithSource";

#[test]
fn delve_is_the_face_flag() {
    let body = read(
        "Name:Delver\nManaCost:7 U\nTypes:Sorcery\nK:Delve\n\
         A:SP$ Draw | NumCards$ 3",
    );
    assert!(body.delve);
    assert_eq!(body.abilities, ["spell!(&[Effect::draw(3)])"]);
}

#[test]
fn a_delve_creature_counts_the_instants_and_sorceries_exiled_with_it() {
    let body = read(&format!(
        "Name:Delver\nManaCost:5 U U\nTypes:Creature Wizard\nPT:3/3\nK:Delve\n{COUNT}"
    ));
    assert!(body.delve);
    assert_eq!(body.enter_modifiers.len(), 1, "{:?}", body.enter_modifiers);
    let entry = &body.enter_modifiers[0];
    assert!(
        entry.starts_with("EnterModifier::WithCounters { kind: CounterKind::P1P1, amount: Amount::CountOf { filter: &")
            && entry.ends_with(", zone: ZoneSel::LinkedExile } }"),
        "{entry}"
    );
    let rendered = format!("{entry}\n{}", body.statics);
    assert!(
        rendered.contains("INSTANT") && rendered.contains("SORCERY"),
        "{rendered}"
    );
}

#[test]
fn a_count_of_cards_exiled_with_a_permanent_without_delve_is_refused() {
    let parsed = parse(&format!(
        "Name:Not a delver\nManaCost:5 U U\nTypes:Creature Wizard\nPT:3/3\n{COUNT}"
    ));
    assert!(transcode(&parsed, &cats(), None).is_none());
    assert_eq!(
        refusal_reason(&parsed, &cats(), None).as_deref(),
        Some("a count of cards exiled with a permanent whose spell has no delve")
    );
}

#[test]
fn a_card_leaving_your_graveyard_is_the_leaves_graveyard_trigger() {
    let body = read(
        "Name:Watcher\nManaCost:1 U\nTypes:Creature Wizard\nPT:1/1\n\
         T:Mode$ ChangesZone | ValidCard$ Instant.YouOwn,Sorcery.YouOwn | Origin$ Graveyard | Destination$ Any | TriggerZones$ Battlefield | Execute$ TrigPutCounter\n\
         SVar:TrigPutCounter:DB$ PutCounter | CounterType$ P1P1 | CounterNum$ 1",
    );
    assert!(
        body.abilities[0].starts_with("triggered!(Trigger::LeavesGraveyard(&TRIGGER"),
        "{:?}",
        body.abilities
    );
    assert!(
        body.statics.contains("Filter::OwnedByYou")
            && body.statics.contains("TypeSet::INSTANT")
            && body.statics.contains("TypeSet::SORCERY"),
        "{}",
        body.statics
    );
}

#[test]
fn the_card_itself_leaving_its_graveyard_is_refused_by_name() {
    let parsed = parse(
        "Name:Watcher\nManaCost:1 U\nTypes:Creature Wizard\nPT:1/1\n\
         T:Mode$ ChangesZone | ValidCard$ Card.Self | Origin$ Graveyard | Destination$ Any | Execute$ TrigGain\n\
         SVar:TrigGain:DB$ GainLife | LifeAmount$ 2",
    );
    assert!(transcode(&parsed, &cats(), None).is_none());
    assert_eq!(
        refusal_reason(&parsed, &cats(), None).as_deref(),
        Some("a card leaving the graveyard it triggers from")
    );
}
