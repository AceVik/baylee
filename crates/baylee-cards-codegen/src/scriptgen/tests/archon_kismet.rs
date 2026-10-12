//! Synthetic exact grammar fixtures; child of scriptgen/tests, not corpus copies.
use super::*;
const QUOTA: &str = "Name:X\nTypes:Creature Wizard\nPT:2/3\nK:Flying\nS:Mode$ CantBeCast | ValidCard$ Card | Caster$ Player | NumLimitEachTurn$ 1\n";
fn entry(valid: &str) -> String {
    format!(
        "Name:X\nTypes:Enchantment\nR:Event$ Moved | ValidCard$ {valid} | Destination$ Battlefield | ActiveZones$ Battlefield | ReplacementResult$ Updated | ReplaceWith$ Entry\nSVar:Entry:DB$ Tap | Defined$ ReplacedCard | ETB$ True\n"
    )
}
#[test]
fn spell_quota_reader_requires_the_exact_each_player_one_per_turn_shape() {
    let body = read(QUOTA);
    let rendered = body.abilities.join(" ");
    assert!(rendered.contains("SpellCastLimitPerTurn"));
    assert!(rendered.contains("PlayerRel::EachPlayer"));
    assert!(rendered.contains("limit: 1"));
    for bad in [
        QUOTA.replace("NumLimitEachTurn$ 1", "NumLimitEachTurn$ 2"),
        QUOTA.replace("Caster$ Player", "Caster$ Opponent"),
        QUOTA.replace("ValidCard$ Card", "ValidCard$ Creature"),
        QUOTA.replace(
            "NumLimitEachTurn$ 1",
            "NumLimitEachTurn$ 1 | Condition$ Unsupported",
        ),
    ] {
        assert!(refused(&bad), "{bad}");
    }
}
#[test]
fn external_entry_reader_preserves_opponent_union_and_nonbasic_supertype() {
    for valid in [
        "Land.OppCtrl+nonBasic",
        "Artifact.OppCtrl,Creature.OppCtrl,Land.OppCtrl",
    ] {
        let body = read(&entry(valid));
        let rendered = format!("{} {}", body.statics, body.abilities.join(" "));
        assert!(rendered.contains("EnterTapped"));
        assert!(rendered.contains("ControlledByOpponent"));
        if valid.contains("nonBasic") {
            assert!(rendered.contains("SupertypeSet::BASIC"));
            assert!(rendered.contains("Filter::Not"));
        } else {
            for kind in ["ARTIFACT", "CREATURE", "LAND"] {
                assert!(rendered.contains(kind), "{rendered}");
            }
        }
    }
    for bad in [
        entry("Land.OppCtrl+nonBasic")
            .replace("Destination$ Battlefield", "Destination$ Graveyard"),
        entry("Land.OppCtrl+nonBasic")
            .replace("ActiveZones$ Battlefield", "ActiveZones$ Graveyard"),
        entry("Land.OppCtrl+nonBasic").replace("ETB$ True", "ETB$ False"),
        entry("Land.OppCtrl+nonBasic").replace("Defined$ ReplacedCard", "Defined$ Self"),
        entry("Land.OppCtrl+nonBasic")
            .replace("ReplacementResult$ Updated", "ReplacementResult$ Replaced"),
        entry("Land.OppCtrl+nonBasic")
            .replace("DB$ Tap", "DB$ Tap | ConditionPresent$ Land.YouCtrl"),
    ] {
        assert!(refused(&bad), "{bad}");
    }
}
