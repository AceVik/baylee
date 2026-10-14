//! The original 133-recording selection, independent of generated manifest rows.
use super::samples::Spec;

// A selection row has the same eight fields as its original authored specification.
#[allow(clippy::too_many_arguments)]
fn add(
    rows: &mut Vec<Spec>,
    family: &str,
    name: &str,
    source: &'static str,
    path: String,
    kind: &'static str,
    note: Option<&str>,
    octave: i16,
) {
    rows.push(Spec {
        family: family.into(),
        name: name.into(),
        source,
        path,
        kind,
        note: note.map(str::to_owned),
        octave,
    });
}

// The selection is data; keeping its rows together makes revision review possible.
#[allow(clippy::too_many_lines)]
pub(super) fn selection() -> Vec<Spec> {
    let mut rows = Vec::new();
    let mut pitched = |family: &str, notes: &str, source, template: &str, kind, octave| {
        for note in notes.split_whitespace() {
            add(
                &mut rows,
                family,
                &format!("{family}-{note}"),
                source,
                template.replace("{}", note),
                kind,
                Some(note),
                octave,
            );
        }
    };
    pitched(
        "alto",
        "F3 G#3 A#3 C4 D4 E4 F#4 G#4",
        "VCSL",
        "Aerophones/Edge-blown Aerophones/Baroque Alto Recorder/Sustain/AltRecorder_Sus_{}_rr1_Main.wav",
        "sus",
        1,
    );
    pitched(
        "tenor",
        "C3 D3 E3 F#3 G#3 A#3 C4",
        "VCSL",
        "Aerophones/Edge-blown Aerophones/Baroque Tenor Recorder/Sustain/TenRecorder_Sus_{}_rr1_Main.wav",
        "sus",
        1,
    );
    for (family, articulation, kind, notes) in [
        (
            "psaltery",
            "Pluck",
            "pluck",
            "A#3:rr1 C4:rr1 D4:rr1 E4:rr2 F#4:rr1 G#4:rr3 A#4:rr1 C5:rr1",
        ),
        ("longbow", "LongBow", "sus", "A#3:rr2 C4:rr1 D4:rr1"),
    ] {
        for entry in notes.split_whitespace() {
            let (note, rr) = entry
                .split_once(':')
                .expect("literal selection note:round-robin");
            add(
                &mut rows,
                family,
                &format!("{family}-{note}"),
                "VCSL",
                format!(
                    "Chordophones/Zithers/Psaltery, Bowed and Plucked/{articulation}/BowedPsaltery_{note}_Main_{articulation}_{rr}.wav"
                ),
                kind,
                Some(note),
                1,
            );
        }
    }
    let mut pitched = |family: &str, notes: &str, source, template: &str, kind, octave| {
        for note in notes.split_whitespace() {
            add(
                &mut rows,
                family,
                &format!("{family}-{note}"),
                source,
                template.replace("{}", note),
                kind,
                Some(note),
                octave,
            );
        }
    };
    pitched(
        "harp",
        "A#1 D2 F#2 A#2 D3 F#3 A#3 D4 F#4",
        "VCSL",
        "Chordophones/Composite Chordophones/Folk Harp/EWHarp_Normal_{}_v3_RR1.wav",
        "pluck",
        1,
    );
    pitched(
        "strumstick",
        "D2 F#2 G2",
        "VCSL",
        "Chordophones/Composite Chordophones/Strumstick/Finger/Strumstick_Finger_Str1_Main_{}_vl2_rr1.wav",
        "pluck",
        1,
    );
    pitched(
        "organ",
        "A#1 C2 F#2",
        "VCSL",
        "Aerophones/Edge-blown Aerophones/Renaissance Organ/8'/RenOrgan_8foot_Room_{}_rr1.wav",
        "drone",
        1,
    );
    let idio = "Idiophones/Struck Idiophones";
    for (note, rr) in [("G#3", "rr2"), ("C4", "rr1")] {
        add(
            &mut rows,
            "chimes",
            &format!("chimes-{note}"),
            "VCSL",
            format!("{idio}/Tubular Bells 1/chimes_{note}_p_{rr}.wav"),
            "bell",
            Some(note),
            1,
        );
    }
    for i in 1..=2 {
        add(
            &mut rows,
            "handbell",
            &format!("handbell-{i}"),
            "VCSL",
            format!("{idio}/Hand Bells, Nepalese/HB_{i}.wav"),
            "bell",
            None,
            0,
        );
    }
    add(
        &mut rows,
        "fingercymbal",
        "fingercymbal",
        "VCSL",
        format!("{idio}/Finger Cymbals/Fing_Cymb.wav"),
        "bell",
        None,
        0,
    );
    let perc = "Membranophones/Struck Membranophones";
    for (family, folder, entries) in [
        (
            "framedrum",
            "Frame Drum",
            "framedrum-1:HDrumL_Hit_v2_rr1_Sum framedrum-2:HDrumL_Hit_v3_rr1_Sum framedrum-muted:HDrumL_HitMuted_v2_rr1_Sum framedrum-small:HDrumS_Hit_v2_rr1_Sum framedrum-small-muted:HDrumS_HitMuted_v2_rr1_Sum",
        ),
        (
            "davul",
            "Bass Drum 1",
            "davul-1:BDrumNew_hit_v3_rr1_Sum davul-2:BDrumNew_hit_v3_rr2_Sum davul-forte:BDrumNew_hit_v5_rr1_Sum",
        ),
        (
            "naker",
            "Timpani 1/Hit",
            "naker-1:Timpani3_Hit_v3_rr1_Sum naker-2:Timpani3_Hit_v3_rr2_Sum naker-high:Timpani4_Hit_v3_rr1_Sum",
        ),
        (
            "ropesnare",
            "Snare Drum, Rope Tension/Low",
            "ropesnare-1:RopeSnare_low_sn_Main_vl2_rr1 ropesnare-2:RopeSnare_low_sn_Main_vl2_rr2",
        ),
    ] {
        for entry in entries.split_whitespace() {
            let (name, file) = entry.split_once(':').expect("literal selection name:file");
            add(
                &mut rows,
                family,
                name,
                "VCSL",
                format!("{perc}/{folder}/{file}.wav"),
                "drum",
                None,
                0,
            );
        }
    }
    add(
        &mut rows,
        "tambourine",
        "tambourine-hit",
        "VCSL",
        format!("{idio}/Tambourine 1/Tamb1_Hit_v2_rr1_Mid.wav"),
        "drum",
        None,
        0,
    );
    let mut pitched = |family: &str, notes: &str, template: &str, kind, octave| {
        for note in notes.split_whitespace() {
            add(
                &mut rows,
                family,
                &format!("{family}-{note}"),
                "VSCO 2 CE",
                template.replace("{}", note),
                kind,
                Some(note),
                octave,
            );
        }
    };
    pitched(
        "contrabass",
        "C1 E1 G#1 A1",
        "Strings/Solo Contrabass/SusNV/BKCtbss_SusNV_{}_v1_rr1.wav",
        "sus",
        1,
    );
    pitched(
        "pizzicato",
        "C1 E1 G#1 A1",
        "Strings/Solo Contrabass/Pizz/BKCtbss_Pizz_{}_v1_rr1.wav",
        "pluck",
        1,
    );
    pitched(
        "cello",
        "B1 D2 F2 A2",
        "Strings/Cello Section/susvib/susvib_{}_v1_1.wav",
        "sus",
        1,
    );
    pitched(
        "violin",
        "G3 C4 E4 G4 A4 C5",
        "Strings/Solo Violin/Arco Vib/LLVln_ArcoVib_{}_p.wav",
        "sus",
        0,
    );
    pitched(
        "horn",
        "D2 F2 A2 C3",
        "Brass/F Horn/sus/MOHorn_sus_{}_v2_1.wav",
        "sus",
        1,
    );
    pitched(
        "violins",
        "D3 F#3 A3 C4 E4",
        "Strings/Violin Section/susVib/VlnEns_susVib_{}_v1.wav",
        "sus",
        1,
    );
    pitched(
        "violas",
        "D2 G2 B2 D3",
        "Strings/Viola Section/susvib/ViolaEns_susvib_{}_v1_1.wav",
        "sus",
        1,
    );
    pitched(
        "violins-spic",
        "D3 F#3 A3 C4",
        "Strings/Violin Section/Spic/VlnEns_Spic_{}_v1_rr1.wav",
        "short",
        1,
    );
    pitched(
        "violas-spic",
        "C2 E2 G2",
        "Strings/Viola Section/spic/Violas_spic_{}_v1_rr1.wav",
        "short",
        1,
    );
    pitched(
        "cellos-spic",
        "G1 B1 D2 F2 A2",
        "Strings/Cello Section/spic/spic_{}_v1_RR1.wav",
        "short",
        1,
    );
    pitched(
        "trombone",
        "A#1 D2 F2 C3",
        "Brass/Tenor Trombone/sus/tenortbn_sus_{}_v2_1.wav",
        "sus",
        1,
    );
    pitched(
        "tuba",
        "A#0 D#1 F1 A#1",
        "Brass/Tuba/sus/Tuba3_sus_{}_v2_rr1_Mid.wav",
        "sus",
        1,
    );
    let vsco1 = "VSCO 1 Percussion";
    for (name, file) in [
        ("taiko-ff", "mallet/EthnicLargeMallet_hit_ff_1"),
        ("taiko-f", "mallet/EthnicLargeMallet_hit_f_1"),
        ("taiko-mf", "mallet/EthnicLargeMallet_hit_mf_1"),
        ("taiko-sticks", "sticks/EthnicLargeSticks_hit_f_1"),
    ] {
        add(
            &mut rows,
            "taiko",
            name,
            "VSCO 2 CE",
            format!("{vsco1}/drums/other/ethnic/giant/{file}.wav"),
            "boom",
            None,
            0,
        );
    }
    add(
        &mut rows,
        "bigdrum",
        "bigdrum-hit",
        "VSCO 2 CE",
        format!("{vsco1}/drums/bass/bdrum_fff_1.wav"),
        "boom",
        None,
        0,
    );
    add(
        &mut rows,
        "bigdrum",
        "bigdrum-roll",
        "VSCO 2 CE",
        format!("{vsco1}/drums/bass/bdrum_roll_long1.wav"),
        "swell",
        None,
        0,
    );
    for (name, folder, file) in [
        ("tom-high", "Tom 1", "TomH_HitM_v3_rr1_Mid"),
        ("tom-low", "Tom 2", "TomL_HitM_v3_rr1_Mid"),
    ] {
        add(
            &mut rows,
            "toms",
            name,
            "VCSL",
            format!("{perc}/{folder}/Mallet/{file}.wav"),
            "drum",
            None,
            0,
        );
    }
    add(
        &mut rows,
        "timpani",
        "timpani-roll",
        "VSCO 2 CE",
        "Percussion/Timpani/Rolls/Timpani2_Roll_v5_rr1_Sum.wav".into(),
        "swell",
        None,
        0,
    );
    add(
        &mut rows,
        "cymbal",
        "cymbal-swell",
        "VSCO 2 CE",
        format!("{vsco1}/varMetal/Cymbals/susp/susp_hit_softmall_roll2_cresc.wav"),
        "swell",
        None,
        0,
    );
    add(
        &mut rows,
        "gong",
        "gong",
        "VCSL",
        format!("{idio}/Gong 1/gong_f.wav"),
        "swell",
        None,
        0,
    );
    for note in "F4 G4 A4 A#4 C5 D5 E5 F5 G5".split_whitespace() {
        let rr = if note == "C5" { 32 } else { 31 };
        add(
            &mut rows,
            "chanter",
            &format!("chanter-{note}"),
            "FreePats Bagpipe",
            format!("samples/{note}_{rr}.flac"),
            "sus",
            Some(note),
            0,
        );
    }
    for (note, rr) in [("G2", 1), ("G3", 3)] {
        add(
            &mut rows,
            "drone",
            &format!("drone-{note}"),
            "FreePats Bagpipe",
            format!("samples/drone_{note}_{rr}.flac"),
            "drone",
            Some(note),
            0,
        );
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::art::samples::{Row, midi_of, revision};

    #[test]
    fn selection_matches_all_committed_source_and_pitch_pins() {
        let rows: Vec<Row> =
            serde_json::from_str(include_str!("../../../../art/music/samples.json")).unwrap();
        let specs = selection();
        assert_eq!(specs.len(), 133);
        assert_eq!(rows.len(), specs.len());
        for spec in specs {
            let row = rows.iter().find(|row| row.name == spec.name).unwrap();
            assert_eq!(
                (
                    &row.family,
                    row.source.as_str(),
                    &row.path,
                    row.kind.as_str()
                ),
                (&spec.family, spec.source, &spec.path, spec.kind)
            );
            assert_eq!(row.revision, revision(spec.source).unwrap());
            assert_eq!(
                row.midi,
                spec.note
                    .as_ref()
                    .map_or(60, |note| midi_of(note).unwrap() + 12 * spec.octave)
            );
        }
    }
}
