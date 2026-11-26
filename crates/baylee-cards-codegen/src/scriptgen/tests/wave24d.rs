use super::*;

fn wave24d_cats() -> SubtypeCatalogs {
    let mut c = SubtypeCatalogs {
        creature: vec![
            "Angel".into(),
            "Cat".into(),
            "Demon".into(),
            "Detective".into(),
            "Dragon".into(),
            "Eldrazi".into(),
            "Goblin".into(),
            "Human".into(),
            "Nightmare".into(),
            "Spirit".into(),
            "Wizard".into(),
            "Zombie".into(),
        ],
        artifact: vec!["Clue".into(), "Spacecraft".into()],
        enchantment: vec!["Aura".into()],
        land: vec!["Forest".into(), "Island".into(), "Mountain".into()],
        ..SubtypeCatalogs::default()
    };
    c.normalize();
    c
}

fn read_w24(text: &str) -> CardBody {
    let parsed = parse(text);
    transcode(&parsed, &wave24d_cats(), None).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}",
            refusal_reason(&parsed, &wave24d_cats(), None)
        )
    })
}

fn refused_w24(text: &str) -> bool {
    transcode(&parse(text), &wave24d_cats(), None).is_none()
}

#[test]
fn move_and_modify_static_effects_and_refusals() {
    // Targeted Graveyard -> Battlefield with StaticEffect$ (AddType + AddColor).
    let rise = read_w24(
        "Name:Rise from the Grave\nManaCost:4 B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | GainControl$ True | ValidTgts$ Creature | StaticEffect$ Animate\n\
         SVar:Animate:Mode$ Continuous | Affected$ Card.IsRemembered | AddType$ Zombie | AddColor$ Black",
    );
    assert!(
        rise.abilities[0].contains("Effect::MoveAndModify")
            && rise.abilities[0].contains("Modifier::AddSubtype(subtypes::creature::ZOMBIE)")
            && rise.abilities[0]
                .contains("Modifier::AddColor(ColorSet::from_slice(&[Color::Black]))")
            && rise.abilities[0].contains("owner_control: false"),
        "{}",
        rise.abilities[0]
    );

    // Dies trigger with RemoveCardTypes$ True -> Modifier::BecomeTypes (Enduring Courage).
    let enduring = read_w24(
        "Name:Enduring Courage\nManaCost:2 R R\nTypes:Enchantment Creature Cat\nPT:3/3\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self+Creature | Execute$ DBReturn\n\
         SVar:DBReturn:DB$ ChangeZone | Defined$ TriggeredNewCardLKICopy | Origin$ Graveyard | Destination$ Battlefield | StaticEffect$ Animate\n\
         SVar:Animate:Mode$ Continuous | Affected$ Card.IsRemembered | AddType$ Enchantment | RemoveCardTypes$ True",
    );
    assert!(
        enduring.abilities[0].contains("Effect::MoveAndModify")
            && enduring.abilities[0].contains("MoveSubject::Target(TargetSpec::EventObject)")
            && enduring.abilities[0]
                .contains("Modifier::BecomeTypes { types: TypeSet::ENCHANTMENT, subtypes: &[] }")
            && enduring.abilities[0].contains("owner_control: true"),
        "{}",
        enduring.abilities[0]
    );

    // Dies trigger with RemoveCreatureTypes$ True -> ReplaceCreatureTypes + AddSubtype (Otherworldly Escort).
    let escort = read_w24(
        "Name:Otherworldly Escort\nManaCost:3 W\nTypes:Creature Human Detective\nPT:4/3\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self+nonSpirit | Execute$ TrigReturn\n\
         SVar:TrigReturn:DB$ ChangeZone | Defined$ TriggeredNewCardLKICopy | Origin$ Graveyard | Destination$ Battlefield | WithCountersType$ CHARGE | WithCountersAmount$ 4 | StaticEffect$ Animate\n\
         SVar:Animate:Mode$ Continuous | Affected$ Card.IsRemembered | AddType$ Spirit & Detective | RemoveCreatureTypes$ True",
    );
    assert!(
        escort.abilities[0].contains("counters: Some((CounterKind::Charge, Amount::Fixed(4)))")
            && escort.abilities[0].contains("Modifier::ReplaceCreatureTypes(subtypes::creature::SPIRIT), Modifier::AddSubtype(subtypes::creature::DETECTIVE)"),
        "{}",
        escort.abilities[0]
    );

    // Layer 6 ordering: RemoveAllAbilities$ True MUST precede AddKeyword$ (Hellcat).
    let hellcat = read_w24(
        "Name:Hellcat\nManaCost:G G\nTypes:Creature Human\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ TrigReturn\n\
         SVar:TrigReturn:DB$ ChangeZone | Defined$ TriggeredNewCardLKICopy | Origin$ Graveyard | Destination$ Battlefield | WithCountersType$ P1P1 | StaticEffect$ Animate\n\
         SVar:Animate:Mode$ Continuous | Affected$ Card.IsRemembered | RemoveAllAbilities$ True | AddKeyword$ Haste",
    );
    let lose_pos = hellcat.abilities[0].find("Modifier::LoseAllAbilities");
    let kw_pos = hellcat.abilities[0].find("Modifier::AddKeyword(KeywordSet::HASTE)");
    assert!(
        lose_pos.is_some() && kw_pos.is_some() && lose_pos < kw_pos,
        "{}",
        hellcat.abilities[0]
    );

    // Transformed + Tapped dies return (Ojer Axonil).
    let ojer = read_w24(
        "Name:Ojer\nManaCost:2 R R\nTypes:Creature Dragon\nPT:4/4\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ TrigTransform\n\
         SVar:TrigTransform:DB$ ChangeZone | Defined$ TriggeredCard | Origin$ Graveyard | Destination$ Battlefield | Transformed$ True | Tapped$ True",
    );
    assert!(
        ojer.abilities[0].contains("Effect::MoveAndModify")
            && ojer.abilities[0].contains("tapped: true, face: Some(1)"),
        "{}",
        ojer.abilities[0]
    );
}

#[test]
fn move_and_modify_all_in_zone_choose_and_refusals() {
    // ChangeZoneAll Graveyard -> Battlefield with StaticEffect$ (Grimoire of the Dead / Storm of Souls).
    let storm = read_w24(
        "Name:Storm of Souls\nManaCost:4 W W\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | ChangeType$ Creature.YouOwn | Origin$ Graveyard | Destination$ Battlefield | StaticEffect$ Animate\n\
         SVar:Animate:Mode$ Continuous | Affected$ Card.IsRemembered | SetPower$ 1 | SetToughness$ 1 | AddType$ Spirit | AddKeyword$ Flying",
    );
    assert!(
        storm.abilities[0].contains("MoveSubject::AllInZone { zone: ZoneSel::GraveyardYou")
            && storm.abilities[0].contains("Modifier::AddSubtype(subtypes::creature::SPIRIT)")
            && storm.abilities[0].contains("Modifier::AddKeyword(KeywordSet::FLYING)")
            && storm.abilities[0].contains("Modifier::SetPT(1, 1)"),
        "{}",
        storm.abilities[0]
    );

    // ChangeZoneAll LinkedExile -> Battlefield with filter + Tapped$.
    let linked = read_w24(
        "Name:Cold Storage\nManaCost:4\nTypes:Artifact\n\
         A:AB$ ChangeZoneAll | Cost$ T Sac<1/CARDNAME> | Origin$ Exile | Destination$ Battlefield | ChangeType$ Creature.ExiledWithSource | Tapped$ True",
    );
    assert!(
        linked.abilities[0].contains("MoveSubject::AllInZone { zone: ZoneSel::LinkedExile")
            && linked.abilities[0].contains("expected_origin: Some(ZoneRef::Exile)")
            && linked.abilities[0].contains("tapped: true"),
        "{}",
        linked.abilities[0]
    );

    // Hidden$ True graveyard choice with WithCountersType$ (Charnel Serenade) lowers via ChooseMove::None + MoveAndModify.
    let serenade = read_w24(
        "Name:Charnel Serenade\nManaCost:4 B B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | WithCountersType$ FINALITY | ChangeType$ Creature.YouOwn | ChangeNum$ 1 | Hidden$ True | Mandatory$ True",
    );
    assert!(
        serenade.abilities[0].contains("movement: ChooseMove::None, then: &[Effect::MoveAndModify")
            && serenade.abilities[0]
                .contains("counters: Some((CounterKind::Finality, Amount::Fixed(1)))"),
        "{}",
        serenade.abilities[0]
    );

    // Remembered reanimate combining head StaticEffect$ + DB$ PutCounter (Call a Surprise Witness).
    let witness = read_w24(
        "Name:Call a Surprise Witness\nManaCost:1 W\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | ValidTgts$ Creature.YouCtrl+cmcLE3 | StaticEffect$ Animate | RememberChanged$ True | SubAbility$ DBPutCounter\n\
         SVar:DBPutCounter:DB$ PutCounter | Defined$ Remembered | CounterType$ Flying | CounterNum$ 1 | SubAbility$ DBCleanup\n\
         SVar:Animate:Mode$ Continuous | Affected$ Card.IsRemembered | AddType$ Spirit\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        witness.abilities[0].contains("Effect::MoveAndModify")
            && witness.abilities[0]
                .contains("counters: Some((CounterKind::Flying, Amount::Fixed(1)))")
            && witness.abilities[0].contains("Modifier::AddSubtype(subtypes::creature::SPIRIT)"),
        "{}",
        witness.abilities[0]
    );

    // REFUSAL: StaticEffect$ with Enchant:<...> (Bronzehide Lion) must be refused.
    assert!(refused_w24(
        "Name:Bronzehide Lion\nManaCost:G W\nTypes:Creature Cat\nPT:3/3\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ DBReturn\n\
         SVar:DBReturn:DB$ ChangeZone | Defined$ TriggeredNewCardLKICopy | Origin$ Graveyard | Destination$ Battlefield | GainControl$ True | StaticEffect$ Animate\n\
         SVar:Animate:Mode$ Continuous | Affected$ Card.IsRemembered | AddType$ Aura & Enchantment | RemoveCardTypes$ True | RemoveAllAbilities$ True | AddKeyword$ Enchant:Creature.YouCtrl:creature you control",
    ));

    // REFUSAL: Unknown parameter on StaticEffect$ SVar must be refused.
    assert!(refused_w24(
        "Name:BadStatic\nManaCost:4 B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | ValidTgts$ Creature | StaticEffect$ Animate\n\
         SVar:Animate:Mode$ Continuous | Affected$ Card.IsRemembered | AddType$ Zombie | BogusKey$ True",
    ));

    // REFUSAL: `Affected$ Creature.IsRemembered` on a dies trigger (could return an animated noncreature manland).
    assert!(refused_w24(
        "Name:LimDul\nManaCost:5 B B\nTypes:Creature Human Wizard\nPT:4/4\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Creature.OppCtrl | Execute$ TrigReturn\n\
         SVar:TrigReturn:DB$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | GainControl$ True | Defined$ TriggeredNewCardLKICopy | StaticEffect$ Animate\n\
         SVar:Animate:Mode$ Continuous | Affected$ Creature.IsRemembered | AddType$ Zombie",
    ));
}

#[test]
fn change_zone_specials_searches_and_refusals() {
    // Command Beacon -> Effect::CommanderToHand.
    let beacon = read_w24(
        "Name:Command Beacon\nTypes:Land\n\
         A:AB$ ChangeZone | Cost$ T Sac<1/CARDNAME> | ChangeType$ Card.IsCommander+YouOwn | ChangeNum$ 1 | Mandatory$ True | Origin$ Command | Destination$ Hand | Hidden$ True",
    );
    assert!(
        beacon.abilities[0].contains("Effect::CommanderToHand"),
        "{}",
        beacon.abilities[0]
    );

    // Coax from the Blind Eternities (`Sideboard,Exile`) -> Effect::WishToHand.
    let coax = read_w24(
        "Name:Coax from the Blind Eternities\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Sideboard,Exile | Destination$ Hand | ChangeType$ Card.Eldrazi+YouOwn | ChangeNum$ 1 | Hidden$ True | Reveal$ True",
    );
    assert!(
        coax.abilities[0].contains("Effect::WishToHand { filter: &"),
        "{}",
        coax.abilities[0]
    );

    // REFUSAL: `Origin$ Sideboard` alone (Burning Wish) must NOT emit WishToHand (which also searches Exile).
    assert!(refused_w24(
        "Name:Burning Wish\nManaCost:1 R\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Reveal$ True | Origin$ Sideboard | Destination$ Hand | ChangeType$ Sorcery.YouOwn | ChangeNum$ 1 | Hidden$ True",
    ));

    // Arc Blade Stack -> Exile with TIME counters -> Effect::ExileSourceWithCounters.
    let arc_blade = read_w24(
        "Name:Arc Blade\nManaCost:3 R R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 2 | SubAbility$ DBChange\n\
         SVar:DBChange:DB$ ChangeZone | Origin$ Stack | Destination$ Exile | WithCountersType$ TIME | WithCountersAmount$ 3",
    );
    assert!(
        arc_blade.abilities[0]
            .contains("Effect::ExileSourceWithCounters { kind: CounterKind::Time, n: 3 }"),
        "{}",
        arc_blade.abilities[0]
    );

    // Mari-style Graveyard -> Exile on dies trigger with counter -> ExileEventFromGraveyardWithCounters.
    let mari = read_w24(
        "Name:Mari\nManaCost:1 B B\nTypes:Creature Human\nPT:3/2\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Creature.OppCtrl | Execute$ DBExile\n\
         SVar:DBExile:DB$ ChangeZone | Defined$ TriggeredCardLKICopy | Origin$ Graveyard | Destination$ Exile | WithCountersType$ HIT | WithCountersAmount$ 1",
    );
    assert!(
        mari.abilities[0]
            .contains("Effect::ExileEventFromGraveyardWithCounters { kind: counters::HIT, n: 1 }"),
        "{}",
        mari.abilities[0]
    );

    // SearchLibraryUpTo with dynamic ChangeNum$ (Nylea's Intervention / X basic lands) + Find::with_counter.
    let search_x = read_w24(
        "Name:SearchX\nManaCost:X G G\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Hand | ChangeType$ Land.Basic | ChangeNum$ X\n\
         SVar:X:Count$xPaid\n\
         Oracle:Search your library for up to X basic land cards, reveal them, put them into your hand, then shuffle.",
    );
    assert!(
        search_x.abilities[0].contains("Effect::SearchLibraryUpTo { filter: &")
            && search_x.abilities[0].contains("count: Amount::X, find: &Find::HAND"),
        "{}",
        search_x.abilities[0]
    );

    let picard = read_w24(
        "Name:Picard\nManaCost:2 W\nTypes:Creature Human\nPT:2/3\n\
         A:AB$ ChangeZone | Cost$ T | Origin$ Library | Destination$ Battlefield | ChangeType$ Spacecraft | ChangeNum$ 1 | WithCountersType$ CHARGE | WithCountersAmount$ 7",
    );
    assert!(
        picard.abilities[0].contains("Find::BATTLEFIELD.with_counter(CounterKind::Charge, 7)"),
        "{}",
        picard.abilities[0]
    );

    // REFUSAL: Mandatory unrestricted `ChangeType$ Card` search with X cards (no "up to") must stay refused.
    assert!(refused_w24(
        "Name:MandatorySearchX\nManaCost:X 3 B B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Hand | ChangeType$ Card | ChangeNum$ X | Mandatory$ True\n\
         SVar:X:Count$xPaid\n\
         Oracle:Search your library for X cards and put them into your hand, then shuffle.",
    ));
}

#[test]
fn change_zone_mandatory_blink_repopulate_and_ego_drain() {
    // Mandatory$ True on targeted ChangeZone (Debtors' Knell) & blink (Flickerwisp).
    let flicker = read_w24(
        "Name:Flickerwisp\nManaCost:1 W W\nTypes:Creature Spirit\nPT:3/1\n\
         T:Mode$ ChangesZone | ValidCard$ Card.Self | Origin$ Any | Destination$ Battlefield | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ ChangeZone | ValidTgts$ Permanent.Other | Mandatory$ True | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True | SubAbility$ DelTrig\n\
         SVar:DelTrig:DB$ DelayedTrigger | Mode$ Phase | Phase$ End of Turn | Execute$ TrigBounce | RememberObjects$ RememberedLKI | SubAbility$ DBCleanup\n\
         SVar:TrigBounce:DB$ ChangeZone | Origin$ Exile | Destination$ Battlefield | Defined$ DelayTriggerRememberedLKI\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        flicker.abilities[0].contains("Effect::ExileAndReturnAtEndStep"),
        "{}",
        flicker.abilities[0]
    );

    // Immediate blink with +1/+1 counter (Planar Incision).
    let incision = read_w24(
        "Name:Planar Incision\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ ChangeZone | ValidTgts$ Artifact,Creature | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True | SubAbility$ DBReturn\n\
         SVar:DBReturn:DB$ ChangeZone | Defined$ Remembered | Origin$ Exile | Destination$ Battlefield | WithCountersType$ P1P1 | WithCountersAmount$ 1 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        incision.abilities[0].contains("Effect::MoveAndModify")
            && incision.abilities[0].contains("blink: true")
            && incision.abilities[0]
                .contains("counters: Some((CounterKind::P1P1, Amount::Fixed(1)))"),
        "{}",
        incision.abilities[0]
    );

    // Repopulate: targeted ShuffleGraveyardMatchingIntoLibrary.
    let repopulate = read_w24(
        "Name:Repopulate\nManaCost:1 G\nTypes:Instant\n\
         A:SP$ ChangeZoneAll | ValidTgts$ Player | Origin$ Graveyard | Destination$ Library | ChangeType$ Creature | Shuffle$ True",
    );
    assert!(
        repopulate.abilities[0].contains(
            "Effect::ShuffleGraveyardMatchingIntoLibrary { who: PlayerRel::Chosen, filter: &Filter::CREATURE }"
        ),
        "{}",
        repopulate.abilities[0]
    );

    // Ego Drain: Hand -> Exile after a prior target.
    let ego = read_w24(
        "Name:Ego Drain\nManaCost:B\nTypes:Sorcery\n\
         A:SP$ Discard | ValidTgts$ Opponent | NumCards$ 1 | DiscardValid$ Card.nonLand | Mode$ RevealYouChoose | SubAbility$ DBExileHand\n\
         SVar:DBExileHand:DB$ ChangeZone | Origin$ Hand | Destination$ Exile | ChangeType$ Card | Mandatory$ True | ChangeNum$ 1",
    );
    assert!(
        ego.abilities[0].contains("Effect::ExileChosenFromHandWithSource(&Filter::Any)"),
        "{}",
        ego.abilities[0]
    );
}

#[test]
fn remembered_chains_copies_haste_and_incubate() {
    // ExileAndCastCopies (Reenact the Crime / Mizzix's Mastery non-overload).
    let cast_copy = read_w24(
        "Name:CastCopy\nManaCost:3 R\nTypes:Sorcery\n\
         A:SP$ ChangeZone | ValidTgts$ Instant.YouOwn,Sorcery.YouOwn | Origin$ Graveyard | Destination$ Exile | RememberChanged$ True | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Valid$ Card.IsRemembered | ValidSA$ Spell | ValidZone$ Exile | CopyCard$ True | WithoutManaCost$ True | Optional$ True | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        cast_copy.abilities[0].contains("Effect::ExileAndCastCopies { graveyard: None }"),
        "{}",
        cast_copy.abilities[0]
    );

    // REFUSAL: ExileAndCastCopies must refuse a target filter that could include lands.
    assert!(refused_w24(
        "Name:BadCastCopy\nManaCost:3 R\nTypes:Sorcery\n\
         A:SP$ ChangeZone | ValidTgts$ Card.YouOwn | Origin$ Graveyard | Destination$ Exile | RememberChanged$ True | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Valid$ Card.IsRemembered | ValidSA$ Spell | ValidZone$ Exile | CopyCard$ True | WithoutManaCost$ True | Optional$ True",
    ));

    // Exile from graveyard + CreateTokenCopy (Anikthea / Ardyn / Blastoneuron).
    let ardyn = read_w24(
        "Name:Ardyn\nManaCost:5 B B B\nTypes:Creature Human\nPT:4/4\n\
         T:Mode$ Phase | Phase$ BeginCombat | ValidPlayer$ You | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ ChangeZone | Origin$ Graveyard | Destination$ Exile | TargetMin$ 0 | TargetMax$ 1 | ValidTgts$ Creature | RememberChanged$ True | SubAbility$ DBCopy\n\
         SVar:DBCopy:DB$ CopyPermanent | Defined$ Remembered | SetPower$ 5 | SetToughness$ 5 | SetColor$ Black | SetCreatureTypes$ Demon | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        ardyn.abilities[0].contains(
            "Effect::exile(TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::EachPlayer))"
        ) && ardyn.abilities[0].contains("ThisWay::ExiledFromGraveyard(&Filter::Any)")
            && ardyn.abilities[0].contains("Effect::CreateTokenCopy")
            && ardyn.abilities[0].contains("CopyMod::SetPT(5, 5)")
            && ardyn.abilities[0]
                .contains("CopyMod::SetCreatureTypes(&[subtypes::creature::DEMON])"),
        "{}",
        ardyn.abilities[0]
    );

    // Wave 28a: Multi-target graveyard exile-and-copy now lowers cleanly via
    // Effect::CaptureAffected + Effect::ForEachRemembered (never ThisWay, so only
    // cards actually moved are copied).
    let multi_copy = read_w24(
        "Name:MultiCopy\nManaCost:5 B B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Exile | TargetMin$ 0 | TargetMax$ 2 | ValidTgts$ Creature | RememberChanged$ True | SubAbility$ DBCopy\n\
         SVar:DBCopy:DB$ CopyPermanent | Defined$ Remembered",
    );
    assert!(
        multi_copy.abilities[0].contains("Effect::CaptureAffected")
            && multi_copy.abilities[0].contains("Effect::ForEachRemembered")
            && multi_copy.abilities[0].contains("Effect::CreateTokenCopy")
            && !multi_copy.abilities[0].contains("ThisWay"),
        "{}",
        multi_copy.abilities[0]
    );

    // Reanimate all + grant haste until EOT -> MoveAndModify with Duration::UntilEndOfTurn (Wake the Past).
    let wake = read_w24(
        "Name:Wake the Past\nManaCost:5 R W\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | Origin$ Graveyard | Destination$ Battlefield | ChangeType$ Artifact.YouOwn | RememberChanged$ True | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ PumpAll | ValidCards$ Card.IsRemembered | KW$ Haste | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        wake.abilities[0].contains("Effect::MoveAndModify")
            && wake.abilities[0].contains("Modifier::AddKeyword(KeywordSet::HASTE)")
            && wake.abilities[0].contains("duration: Duration::UntilEndOfTurn"),
        "{}",
        wake.abilities[0]
    );

    // Sunfall: Exile all creatures + Incubate X (Remembered$Amount).
    let sunfall = read_w24(
        "Name:Sunfall\nManaCost:3 W W\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | Origin$ Battlefield | Destination$ Exile | ChangeType$ Creature | RememberChanged$ True | SubAbility$ DBIncubate\n\
         SVar:DBIncubate:DB$ Incubate | Amount$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount",
    );
    assert!(
        sunfall.abilities[0].contains("Effect::Incubate { who: PlayerRel::You, count: Amount::ThisWay(ThisWay::ExiledFromBattlefield(&Filter::Any)) }"),
        "{}",
        sunfall.abilities[0]
    );

    // Cruel Grimnarch: each opponent discards 1, for each who can't gain 4 life.
    let grimnarch = read_w24(
        "Name:Cruel Grimnarch\nManaCost:5 B\nTypes:Creature Human\nPT:5/5\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDiscard\n\
         SVar:TrigDiscard:DB$ Discard | Mode$ TgtChoose | Defined$ Opponent | RememberDiscarded$ True | SubAbility$ DBGainLife\n\
         SVar:DBGainLife:DB$ GainLife | LifeAmount$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:SVar$Y/Times.4\n\
         SVar:Y:PlayerCountOpponents$Amount/Minus.Remembered$Amount",
    );
    assert!(
        grimnarch.abilities[0].contains("Amount::Times { base: &Amount::Difference { left: &Amount::Players(PlayerRel::EachOpponent), right: &Amount::ThisWay(ThisWay::Discarded) }, factor: 4 }"),
        "{}",
        grimnarch.abilities[0]
    );
}

#[test]
fn exile_and_cast_copies_hoisted_filter_and_dryad_arbor_refusal() {
    // Hoisted nonland filter (Narset / Reenact the Crime) with Defined$ Remembered + Controller$ You.
    let hoisted = read_w24(
        "Name:NarsetStyle\nManaCost:1 U R W\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Card.nonCreature+nonLand+cmcLE3 | RememberChanged$ True | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Defined$ Remembered | CopyCard$ True | Controller$ You | WithoutManaCost$ True | Optional$ True | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        hoisted.abilities[0].contains("Effect::ExileAndCastCopies { graveyard: None }"),
        "{}",
        hoisted.abilities[0]
    );

    // REFUSAL: Creature in graveyard can match Dryad Arbor (a Land Creature) and must NOT emit ExileAndCastCopies.
    assert!(refused_w24(
        "Name:BadDryadCopy\nManaCost:3 R\nTypes:Sorcery\n\
         A:SP$ ChangeZone | ValidTgts$ Creature.YouOwn | Origin$ Graveyard | Destination$ Exile | RememberChanged$ True | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Valid$ Card.IsRemembered | ValidSA$ Spell | ValidZone$ Exile | CopyCard$ True | WithoutManaCost$ True | Optional$ True",
    ));
}

#[test]
fn pump_double_freeze_hexproof_protection_and_refusals() {
    // Single-target Pump Double -> Amount::TargetPower / Amount::TargetToughness.
    let double_pt = read_w24(
        "Name:Legion Leadership\nManaCost:1 RW\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ Double | NumDef$ Double\n\
         Oracle:Double target creature's power and toughness until end of turn.",
    );
    assert!(
        double_pt.abilities[0]
            .contains("power: Amount::TargetPower, toughness: Amount::TargetToughness"),
        "{}",
        double_pt.abilities[0]
    );

    // Wave 17c: Double on Self, Valid, and multi-target lower via Effect::DoublePT;
    // Alchemy Duration$ Perpetual remains refused.
    assert!(
        read_w24(
            "Name:SelfDouble\nManaCost:1 G\nTypes:Creature Cat\nPT:2/2\n\
             A:AB$ Pump | Cost$ G | Defined$ Self | NumAtt$ Double",
        )
        .abilities[0]
            .contains("Effect::DoublePT")
    );
    assert!(
        read_w24(
            "Name:ValidDouble\nManaCost:3 G\nTypes:Sorcery\n\
             A:SP$ Pump | Defined$ Valid Creature.YouCtrl | NumAtt$ Double",
        )
        .abilities[0]
            .contains("Effect::DoublePT")
    );
    assert!(
        read_w24(
            "Name:MultiDouble\nManaCost:2 G\nTypes:Instant\n\
             A:SP$ Pump | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ 2 | NumAtt$ Double\n\
             Oracle:Double the power of up to two target creatures until end of turn.",
        )
        .abilities[0]
            .contains("Effect::DoublePT")
    );
    assert!(refused_w24(
        "Name:AlchemyPerp\nManaCost:B\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ Double | Duration$ Perpetual",
    ));

    // Undiscovered Paradise -> Effect::ReturnSelfDuringNextUntap.
    let paradise = read_w24(
        "Name:Undiscovered Paradise\nTypes:Land\n\
         A:AB$ Mana | Cost$ T | Produced$ Any | SubAbility$ DBReturn\n\
         SVar:DBReturn:DB$ Pump | Defined$ Self | KW$ HIDDEN During your next untap step, as you untap your permanents, return CARDNAME to its owner's hand. | Duration$ Permanent",
    );
    assert!(
        paradise.abilities[0].contains("Effect::ReturnSelfDuringNextUntap"),
        "{}",
        paradise.abilities[0]
    );

    // SuppressUntap on TapsForMana (Vorinclex), AttackerBlocked (Wall of Frost), DamageDone, Self & Targeted (Dragon Turtle).
    let wall_frost = read_w24(
        "Name:Wall of Frost\nManaCost:1 U U\nTypes:Creature\nPT:0/7\n\
         T:Mode$ AttackerBlocked | ValidBlocker$ Card.Self | Execute$ TrigFreeze\n\
         SVar:TrigFreeze:DB$ Pump | Defined$ TriggeredAttacker | KW$ HIDDEN This card doesn't untap during your next untap step. | Duration$ Permanent",
    );
    assert!(
        wall_frost.abilities[0]
            .contains("Effect::SuppressUntap { target: TargetSpec::EventObject, steps: 1 }"),
        "{}",
        wall_frost.abilities[0]
    );

    let dragon_turtle = read_w24(
        "Name:Dragon Turtle\nManaCost:1 U U\nTypes:Creature Dragon\nPT:3/5\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigTap\n\
         SVar:TrigTap:DB$ Tap | ValidTgts$ Creature.OppCtrl | SubAbility$ DBFreeze\n\
         SVar:DBFreeze:DB$ Pump | Defined$ Self & Targeted | KW$ HIDDEN This card doesn't untap during your next untap step. | Duration$ Permanent",
    );
    assert!(
        dragon_turtle.abilities[0]
            .contains("Effect::SuppressUntap { target: TargetSpec::ThisObject, steps: 1 }")
            && dragon_turtle.abilities[0]
                .contains("Effect::SuppressUntap { target: TargetSpec::Object(&"),
        "{}",
        dragon_turtle.abilities[0]
    );
}

#[test]
fn pump_hexproof_protection_pump_all_and_refusals() {
    // Player hexproof: Defined$ Player and Defined$ You & Valid Permanent.YouCtrl.
    let truce = read_w24(
        "Name:Dawn's Truce\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ Pump | Defined$ You & Valid Permanent.YouCtrl | KW$ Hexproof",
    );
    assert!(
        truce.abilities[0].contains("Modifier::PlayerHexproof { who: PlayerRel::You }")
            && truce.abilities[0].contains("keywords: KeywordSet::HEXPROOF"),
        "{}",
        truce.abilities[0]
    );

    // Protection from each color & Duration$ Permanent Protection from blue & Landwalk:Land.
    let eldritch = read_w24(
        "Name:Eldritch Immunity\nManaCost:C\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | KW$ Protection from each color",
    );
    assert!(
        eldritch.abilities[0].contains("Modifier::ProtectionFrom(&")
            && eldritch
                .statics
                .contains("Filter::Not(&Filter::IsColorless)"),
        "{} / {}",
        eldritch.abilities[0],
        eldritch.statics
    );

    let guardian = read_w24(
        "Name:Council Guardian\nManaCost:5 W\nTypes:Creature\nPT:5/5\n\
         A:AB$ Pump | Cost$ W | KW$ Protection from blue | Duration$ Permanent | Defined$ Self",
    );
    assert!(
        guardian.abilities[0]
            .contains("Effect::ModifySource { modifier: Modifier::ProtectionFrom(&")
            && guardian.abilities[0].contains("duration: Duration::Indefinitely"),
        "{}",
        guardian.abilities[0]
    );

    // PumpAll: SwitchPT (Mannichi), UntilUntaps (Thran Weaponry), TargetedOrController (Concussive Bolt), Landwalk.
    let mannici = read_w24(
        "Name:Mannichi\nManaCost:2 R\nTypes:Creature Spirit\nPT:1/2\n\
         A:AB$ PumpAll | Cost$ 1 R | ValidCards$ Creature | KW$ HIDDEN CARDNAME's power and toughness are switched",
    );
    assert!(
        mannici.abilities[0].contains(
            "Effect::continuous(&Filter::CREATURE, Modifier::SwitchPT, Duration::UntilEndOfTurn)"
        ),
        "{}",
        mannici.abilities[0]
    );

    let weaponry = read_w24(
        "Name:Thran Weaponry\nManaCost:4\nTypes:Artifact\n\
         A:AB$ PumpAll | Cost$ 2 T | ValidCards$ Creature | NumAtt$ +2 | NumDef$ +2 | Duration$ UntilUntaps",
    );
    assert!(
        weaponry.abilities[0]
            .contains("Duration::WhileSourceTapped { control: false, power_limited: false }"),
        "{}",
        weaponry.abilities[0]
    );

    let concussive = read_w24(
        "Name:Concussive Bolt\nManaCost:3 R R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Player,Planeswalker | NumDmg$ 4 | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ PumpAll | Defined$ TargetedOrController | ValidCards$ Creature | KW$ HIDDEN CARDNAME can't block.",
    );
    assert!(
        concussive.abilities[0]
            .contains("controlled_by: Some(PlayerRel::TargetPlayerOrController)")
            && concussive.abilities[0].contains("keywords: KeywordSet::CANT_BLOCK"),
        "{}",
        concussive.abilities[0]
    );

    let searo = read_w24(
        "Name:Searo\nTypes:Land\n\
         A:AB$ PumpAll | Cost$ T | ValidCards$ Creature.YouCtrl | KW$ Landwalk:Land",
    );
    assert!(
        searo.abilities[0].contains("Modifier::LandwalkMatching(&Filter::LAND)"),
        "{}",
        searo.abilities[0]
    );

    // REFUSAL: Sleep-style PumpAll freeze with Defined$ Targeted must stay refused.
    assert!(refused_w24(
        "Name:Sleep\nManaCost:2 U U\nTypes:Sorcery\n\
         A:SP$ TapAll | ValidTgts$ Player | ValidCards$ Creature | SubAbility$ DBFreeze\n\
         SVar:DBFreeze:DB$ PumpAll | Defined$ Targeted | ValidCards$ Creature | KW$ HIDDEN This card doesn't untap during your next untap step. | Duration$ Permanent",
    ));
}

#[test]
fn wave24d_amounts_and_repeat_each_scoped_counts() {
    // Corrupt: Count$TotalDamageDoneByThisTurn on a spell -> Amount::ThisWay(ThisWay::DamageDealt).
    let corrupt = read_w24(
        "Name:Corrupt\nManaCost:5 B\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ X | SubAbility$ DBGainLife\n\
         SVar:DBGainLife:DB$ GainLife | Defined$ You | LifeAmount$ Y\n\
         SVar:X:Count$Valid Island.YouCtrl\n\
         SVar:Y:Count$TotalDamageDoneByThisTurn",
    );
    assert!(
        corrupt.abilities[0]
            .contains("Effect::GainLife { amount: Amount::ThisWay(ThisWay::DamageDealt) }"),
        "{}",
        corrupt.abilities[0]
    );

    // REFUSAL: Count$TotalDamageDoneByThisTurn on a permanent activated ability (could include prior combat damage).
    assert!(refused_w24(
        "Name:BadPermDamageGain\nManaCost:2 B\nTypes:Creature Human\nPT:2/2\n\
         A:AB$ GainLife | Cost$ T | Defined$ You | LifeAmount$ Y\n\
         SVar:Y:Count$TotalDamageDoneByThisTurn",
    ));

    // Divine Congregation: TargetedPlayer$CreaturesInPlay/Times.2.
    let cong = read_w24(
        "Name:Divine Congregation\nManaCost:3 W\nTypes:Sorcery\n\
         A:SP$ GainLife | LifeAmount$ X | ValidTgts$ Player | Defined$ You\n\
         SVar:X:TargetedPlayer$CreaturesInPlay/Times.2",
    );
    assert!(
        cong.abilities[0].contains("Amount::Times { base: &Amount::ControlledCount { who: PlayerRel::Chosen, filter: &Filter::CREATURE }, factor: 2 }"),
        "{}",
        cong.abilities[0]
    );

    // Power Surge: TriggeredPlayer$NumPowerSurgeLands.
    let surge = read_w24(
        "Name:Power Surge\nManaCost:R R\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ Player | Execute$ TrigDmg\n\
         SVar:TrigDmg:DB$ DealDamage | Defined$ TriggeredPlayer | NumDmg$ X\n\
         SVar:X:TriggeredPlayer$NumPowerSurgeLands",
    );
    assert!(
        surge.abilities[0].contains("amount: Amount::UntappedLandsAtTurnStart"),
        "{}",
        surge.abilities[0]
    );

    // Engulfing Slagwurm: TriggeredAttacker$CardToughness / TriggeredBlocker$CardToughness.
    let slagwurm = read_w24(
        "Name:Engulfing Slagwurm\nManaCost:5 G G\nTypes:Creature\nPT:7/7\n\
         T:Mode$ AttackerBlockedByCreature | ValidCard$ Creature | ValidBlocker$ Card.Self | Execute$ TrigGain1\n\
         T:Mode$ AttackerBlockedByCreature | ValidCard$ Card.Self | ValidBlocker$ Creature | Execute$ TrigGain2 | Secondary$ True\n\
         SVar:TrigGain1:DB$ GainLife | Defined$ You | LifeAmount$ X\n\
         SVar:TrigGain2:DB$ GainLife | Defined$ You | LifeAmount$ Y\n\
         SVar:X:TriggeredAttacker$CardToughness\n\
         SVar:Y:TriggeredBlocker$CardToughness",
    );
    assert!(
        slagwurm.abilities[0].contains("amount: Amount::EventLastToughness"),
        "{}",
        slagwurm.abilities[0]
    );
}

#[test]
fn wave24d_zone_counts_recursive_svar_and_repeat_each() {
    // Guiltfeeder: Count$ValidGraveyard Card.DefenderCtrl.
    let guiltfeeder = read_w24(
        "Name:Guiltfeeder\nManaCost:3 B B\nTypes:Creature\nPT:0/4\n\
         T:Mode$ AttackerUnblocked | ValidCard$ Card.Self | Execute$ TrigLose\n\
         SVar:TrigLose:DB$ LoseLife | Defined$ TriggeredDefendingPlayer | LifeAmount$ X\n\
         SVar:X:Count$ValidGraveyard Card.DefenderCtrl",
    );
    assert!(
        guiltfeeder.abilities[0].contains("zone: ZoneSel::GraveyardOf(PlayerRel::DefendingPlayer)"),
        "{}",
        guiltfeeder.abilities[0]
    );

    // Beacon Bolt: Count$ValidGraveyard,Exile Instant.YouOwn,Sorcery.YouOwn.
    let beacon_bolt = read_w24(
        "Name:Beacon Bolt\nManaCost:1 U R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ X\n\
         SVar:X:Count$ValidGraveyard,Exile Instant.YouOwn,Sorcery.YouOwn",
    );
    assert!(
        beacon_bolt.abilities[0].contains("ZoneSel::GraveyardOf(PlayerRel::You)")
            && beacon_bolt.abilities[0].contains("ZoneSel::ExileAll"),
        "{}",
        beacon_bolt.abilities[0]
    );

    // Recursive SVar composition: SVar$A/Plus.B.
    let sum_svar = read_w24(
        "Name:SumSvar\nManaCost:2 R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ Z\n\
         SVar:X:Count$Valid Island.YouCtrl\n\
         SVar:Y:Count$Valid Mountain.YouCtrl\n\
         SVar:Z:SVar$X/Plus.Y",
    );
    assert!(
        sum_svar.abilities[0].contains("Amount::Sum { left: &Amount::CountOf"),
        "{}",
        sum_svar.abilities[0]
    );

    // Angrath & Dire Fleet Ravager: RepeatEach graveyard count + /ThirdUp.
    let angrath = read_w24(
        "Name:Angrath\nManaCost:3 B R\nTypes:Sorcery\n\
         A:SP$ RepeatEach | RepeatPlayers$ Player.Opponent | RepeatSubAbility$ DBLose\n\
         SVar:DBLose:DB$ LoseLife | Defined$ Player.IsRemembered | LifeAmount$ X\n\
         SVar:X:Count$ValidGraveyard Card.RememberedPlayerCtrl",
    );
    assert!(
        angrath.abilities[0].contains("zone: ZoneSel::GraveyardOf(PlayerRel::EventPlayer)"),
        "{}",
        angrath.abilities[0]
    );

    let ravager = read_w24(
        "Name:Dire Fleet Ravager\nManaCost:3 B B\nTypes:Creature\nPT:4/4\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigLose\n\
         SVar:TrigLose:DB$ RepeatEach | RepeatPlayers$ Player | RepeatSubAbility$ DBLose\n\
         SVar:DBLose:DB$ LoseLife | Defined$ Player.IsRemembered | LifeAmount$ CntLife\n\
         SVar:CntLife:PlayerCountRemembered$LifeTotal/ThirdUp",
    );
    assert!(
        ravager.abilities[0]
            .contains("Amount::divided(&Amount::LifeTotal(PlayerRel::EventPlayer), 3, true)"),
        "{}",
        ravager.abilities[0]
    );
}
