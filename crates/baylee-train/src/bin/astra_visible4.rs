//! CPU-only rendering of two measured spell plans from the deciding seat's
//! pre-action replay. The record index deserializes no scores or endings.
use anyhow::{Context, Result, bail, ensure};
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_engine::choice::PlayerAction;
use baylee_gamehost::{
    Session,
    record::{Line, replay_exact},
};
use baylee_train::netplay4::search::visible_spell_plan;
use baylee_view::{PlayerView, PublicObject};
use flate2::read::GzDecoder;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::Path;

#[derive(Deserialize)]
struct Game {
    i: u64,
    focal_seat: u8,
    record_sha256: String,
    shard: String,
    offset: u64,
    len: u64,
    search_events: Vec<Event>,
}
#[derive(Deserialize)]
struct Event {
    seat: u8,
    turn: u32,
    decision_seq: u64,
    record_byte_offset: usize,
    record_input_n: u64,
    candidates: Vec<PlayerAction>,
    scripts: Vec<Option<PlayerAction>>,
    #[serde(default)]
    planned_actions: Option<Vec<Option<Vec<PlayerAction>>>>,
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read<T: serde::de::DeserializeOwned>(p: &Path) -> Result<T> {
    Ok(serde_json::from_reader(File::open(p)?)?)
}
fn write(p: &Path, v: &Value) -> Result<String> {
    let mut bytes = serde_json::to_vec_pretty(v)?;
    bytes.push(b'\n');
    File::create_new(p)?.write_all(&bytes)?;
    Ok(sha(&bytes))
}
fn raw(dir: &Path, g: &Game) -> Result<Vec<u8>> {
    ensure!(
        !g.shard.contains('/') && g.shard.ends_with(".jsonl.gz"),
        "unexpected shard name"
    );
    let mut shard = File::open(dir.join("records").join(&g.shard))?;
    shard.seek(SeekFrom::Start(g.offset))?;
    let mut bytes = Vec::new();
    GzDecoder::new(shard.take(g.len)).read_to_end(&mut bytes)?;
    ensure!(
        sha(&bytes) == g.record_sha256,
        "record SHA mismatch for {}",
        g.i
    );
    Ok(bytes)
}
fn seat_public(o: &PublicObject) -> Value {
    let rules = o
        .rules
        .and_then(|r| baylee_cards::oracle::face(r.card, usize::from(r.face)));
    json!({"object":o.id,"name":o.name,"controller":o.controller,"owner":o.owner,
       "tapped":o.status.is_tapped(),"face_down":o.status.is_face_down(),"summoning_sick":o.summoning_sick,
       "power":o.power,"toughness":o.toughness,"damage":o.damage,"loyalty":o.loyalty,"counters":o.counters,
       "attached_to":o.attached_to,"printed_rules":rules})
}
fn hand(v: &PlayerView) -> Vec<Value> {
    v.hand.iter().map(|c|json!({"object":c.id,"name":c.name,"card_text":baylee_seat::lookup::compiled(&c.name)})).collect()
}
fn state(v: &PlayerView, opening: &[Value]) -> Value {
    let zones = |rows: &[Vec<PublicObject>]| {
        rows.iter()
            .map(|z| z.iter().map(seat_public).collect::<Vec<_>>())
            .collect::<Vec<_>>()
    };
    let players=v.seats.iter().map(|s|json!({"seat":s.player,"life":s.life,"poison":s.poison,"energy":s.energy,"cards_in_hand":s.hand_count,"library_size":s.library_count,"floating_mana":s.mana_pool})).collect::<Vec<_>>();
    json!({"seat":v.seat,"turn":v.turn,"phase":v.phase,"step":v.step,"active_seat":v.active,"players":players,
        "own_opening_hand":opening,"own_hand":hand(v),"battlefield":v.battlefield.iter().map(seat_public).collect::<Vec<_>>(),
        "stack":v.stack.iter().map(seat_public).collect::<Vec<_>>(),"graveyards":zones(&v.graveyards),
        "exile":zones(&v.exile),"command":zones(&v.command),"combat":v.combat,"monarch":v.monarch,"day_night":v.day_night})
}
fn render_choice(v: &PlayerView, plan: &[PlayerAction]) -> Result<Value> {
    let Some(PlayerAction::CastSpell { card }) = plan.last() else {
        bail!("no final spell")
    };
    let name = v
        .hand
        .iter()
        .find(|c| c.id == *card)
        .map(|c| c.name.clone())
        .or_else(|| {
            v.graveyards
                .iter()
                .flatten()
                .chain(v.exile.iter().flatten())
                .chain(v.command.iter().flatten())
                .find(|o| o.id == *card)
                .map(|o| o.name.to_string())
        })
        .context("casting source must be visible")?;
    let source = |id: &ObjectId| {
        v.battlefield
            .iter()
            .find(|o| o.id == *id)
            .map(|o| o.name.to_string())
            .context("source not on visible battlefield")
    };
    let mut payment = Vec::new();
    for a in &plan[..plan.len() - 1] {
        payment.push(match a{
        PlayerAction::ActivateManaAbility{source:id}=>json!({"action":"activate_mana_ability","object":id,"name":source(id)?}),
        PlayerAction::ActivateAbility{source:id,ability_index}=>json!({"action":"activate_ability","object":id,"name":source(id)?,"ability_index":ability_index}),
        PlayerAction::ChooseColor(color)=>json!({"action":"choose_color_if_asked","color":color}),
        _=>bail!("unexpected payment action"),
    });
    }
    ensure!(!payment.is_empty(), "empty payment");
    Ok(
        json!({"spell":name,"card_object":card,"card_text":baylee_seat::lookup::compiled(&name).context("compiled card rules")?,"payment":payment,"then":"cast_spell"}),
    )
}
fn packet(dir: &Path, g: &Game, e: &Event, cols: [usize; 2]) -> Result<(Value, Value)> {
    let bytes = raw(dir, g)?;
    let offset = e.record_byte_offset;
    ensure!(
        offset < bytes.len() && (offset == 0 || bytes[offset - 1] == b'\n'),
        "bad input offset"
    );
    let next = bytes[offset..]
        .split(|&b| b == b'\n')
        .next()
        .context("root input")?;
    let Line::Input { n, seat, .. } = serde_json::from_slice(next)? else {
        bail!("root is not recorded input")
    };
    ensure!(
        n == e.record_input_n && seat == e.seat && seat == g.focal_seat,
        "root input mismatch"
    );
    // The verified prefix stops before the action recorded at the chosen root.
    let prefix = &bytes[..offset];
    let _verified = replay_exact(prefix).context("engine hash verification on prefix")?;
    let mut lines = prefix.split(|&b| b == b'\n').filter(|l| !l.is_empty());
    let Line::Header { preset, .. } =
        serde_json::from_slice(lines.next().context("missing header")?)?
    else {
        bail!("missing header")
    };
    let who = PlayerId::new(seat);
    let mut session = Session::new(&preset).context("replay build")?;
    for i in 0..preset.seats.len() {
        session.take_over(PlayerId::new(i as u8));
    }
    let opening = hand(&session.observe_view_with_context(who));
    for line in lines {
        if let Line::Input { seat, action, .. } = serde_json::from_slice(line)? {
            session
                .act(PlayerId::new(seat), action)
                .map_err(|err| anyhow::anyhow!("input refused: {err}"))?;
        }
    }
    let (pending, v) = session
        .view_for_with_announcement_context(who)
        .context("wrong seat at root")?;
    ensure!(
        v.seat == who
            && v.turn == e.turn
            && v.decision_player.is_none_or(|p| p == who)
            && session.decision_seq() == e.decision_seq,
        "replay identity mismatch"
    );
    let mut options = Vec::new();
    let mut identities = BTreeSet::new();
    for index in cols {
        let Some(PlayerAction::CastSpell { card }) = &e.scripts[index] else {
            bail!("not a planned spell")
        };
        let plan =
            visible_spell_plan(&v, &pending, *card).context("public plan not reconstructible")?;
        ensure!(
            plan.first() == e.candidates.get(index),
            "archived first payment differs"
        );
        if let Some(archived) = &e.planned_actions {
            ensure!(
                archived[index].as_ref() == Some(&plan),
                "archived full payment differs"
            );
        }
        let option = render_choice(&v, &plan)?;
        ensure!(
            identities.insert(option["spell"].as_str().unwrap().to_owned()),
            "duplicate card identity"
        );
        options.push((index, option));
    }
    let public = state(&v, &opening);
    // Randomize the displayed order reproducibly, using only the visible
    // board and the two public plans, sorted independently of event order.
    options.sort_by_key(|(_, o)| o["spell"].as_str().unwrap().to_owned());
    let fingerprint = json!({"state":public,"choices":[&options[0].1,&options[1].1]});
    let digest = Sha256::digest(serde_json::to_vec(&fingerprint)?);
    if digest[0] & 1 == 1 {
        options.swap(0, 1);
    }
    let id = format!("p-{}", &format!("{digest:x}")[..20]);
    Ok((
        json!({"position_id":id,"public_state":public,"choices":{"A":&options[0].1,"B":&options[1].1}}),
        json!({"position_id":id,"deal_index":g.i,"seat":seat,"decision_seq":e.decision_seq,"record_byte_offset":offset,
        "record_sha256":g.record_sha256,"candidate_by_choice":{"A":options[0].0,"B":options[1].0}}),
    ))
}
fn main() -> Result<()> {
    let a: Vec<_> = std::env::args().collect();
    ensure!(
        a.len() == 6,
        "archive-preview|fixed RECORDS FIXTURES_OR_DASH PUBLIC_JSON PRIVATE_JSON"
    );
    let preview = a[1] == "archive-preview";
    ensure!(preview || a[1] == "fixed", "mode");
    let dir = Path::new(&a[2]);
    let run: Value = read(&dir.join("run.json"))?;
    ensure!(
        run["deck_split"] == "train" && run["generated_deck_count"] == 0,
        "not a real TRAIN split"
    );
    let mut games = BTreeMap::new();
    for line in BufReader::new(File::open(dir.join("games.jsonl"))?).lines() {
        let g: Game = serde_json::from_str(&line?)?;
        ensure!(games.insert(g.i, g).is_none(), "duplicate deal");
    }
    if !preview {
        let fixed: Vec<Value> = read(Path::new(&a[3]))?;
        ensure!(
            games
                .keys()
                .copied()
                .eq(fixed.iter().filter_map(|f| f["deal"].as_u64())),
            "incomplete fixtures"
        );
        ensure!(
            run["search"]["payable_pair"] == "policy_anchor_then_two_distinct_payable_spells",
            "unknown macro case"
        );
    }
    let mut packets = Vec::new();
    let mut joins = Vec::new();
    let mut ids = BTreeSet::new();
    'outer: for g in games.values() {
        if !preview {
            ensure!(g.search_events.len() <= 1, "more than one decision");
        }
        for e in &g.search_events {
            if e.turn < 6 {
                continue;
            }
            let cols = if preview {
                let eligible: Vec<_> = e
                    .scripts
                    .iter()
                    .enumerate()
                    .filter_map(|(i, a)| {
                        matches!(a, Some(PlayerAction::CastSpell { .. })).then_some(i)
                    })
                    .take(2)
                    .collect();
                let [i, j] = eligible.as_slice() else {
                    continue;
                };
                [*i, *j]
            } else {
                ensure!(e.scripts.len() == 3, "wrong macro case");
                [1, 2]
            };
            let (p, j) = packet(dir, g, e, cols)?;
            ensure!(
                ids.insert(p["position_id"].as_str().unwrap().to_owned()),
                "duplicate public ID"
            );
            packets.push(p);
            joins.push(j);
            if preview {
                break 'outer;
            }
        }
    }
    ensure!(!packets.is_empty(), "no payable pairs");
    let p = write(Path::new(&a[4]), &json!({"positions":packets}))?;
    write(
        Path::new(&a[5]),
        &json!({"schema":"baylee_astra_macro_join_v1","packet_sha256":p,"positions":joins}),
    )?;
    println!("{} score-blind positions; packet {}", packets.len(), p);
    Ok(())
}
