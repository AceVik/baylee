use super::{FAMILIES, MATERIALS, Result, Tier, tiers};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use naga::{ShaderStage, back::glsl, valid};
use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};

const SHADER: &str = include_str!("../../../../art/baylee/ranks/finish.wgsl");
const MATERIAL: [(&str, u8); MATERIALS.len()] = [
    ("Warmes Streiflicht betont die geknickten Papierfasern.", 0),
    (
        "Eine matte Wachsschicht hebt die geschnitzte Maserung hervor.",
        0,
    ),
    (
        "Ein gebrochener, blaugrauer Reflex streift das gebürstete Eisen.",
        10,
    ),
    (
        "Eine kupferwarme Kante wandert über das Relief. In den Vertiefungen bleibt grüne Patina.",
        9,
    ),
    (
        "Eine schmale weiße Lichtkante zieht einen weicheren Silberreflex hinter sich her.",
        8,
    ),
    (
        "Ein breiter Honigglanz wandert über die erhabenen Sonnenstrahlen.",
        8,
    ),
    (
        "Zwei versetzte Reflexe trennen das kühle Platin vom türkisfarbenen Email.",
        9,
    ),
    (
        "Benachbarte blaue Facetten brechen das Licht zeitversetzt.",
        7,
    ),
    (
        "Der Amethyst leuchtet langsam von innen auf. Eine feine Reflexion trifft die Goldfassung.",
        10,
    ),
    (
        "Zwei rubinrote Lichtbögen laufen durch den Stein und treffen die roségoldene Fassung.",
        9,
    ),
    (
        "Perlmutt wechselt sanft zwischen Rosé, Eisblau und Opalgrün. Der Goldrand fängt warmes Licht.",
        12,
    ),
];

fn stages() -> Result<[String; 2]> {
    let module = naga::front::wgsl::parse_str(SHADER).map_err(|err| err.emit_to_string(SHADER))?;
    let info = valid::Validator::new(valid::ValidationFlags::all(), valid::Capabilities::empty())
        .validate(&module)
        .map_err(|err| err.emit_to_string(SHADER))?;
    let options = glsl::Options {
        version: glsl::Version::new_gles(300),
        writer_flags: glsl::WriterFlags::empty(),
        ..Default::default()
    };
    let stage = |shader_stage, name: &str| -> Result<String> {
        let pipeline = glsl::PipelineOptions {
            shader_stage,
            entry_point: name.into(),
            multiview: None,
        };
        let mut source = String::new();
        glsl::Writer::new(
            &mut source,
            &module,
            &info,
            &options,
            &pipeline,
            naga::proc::BoundsCheckPolicies::default(),
        )?
        .write()?;
        Ok(source)
    };
    Ok([
        stage(ShaderStage::Vertex, "vs")?,
        stage(ShaderStage::Fragment, "fs")?,
    ])
}

fn variants(folder: &Path, tier: &Tier, family: &str) -> Result<Vec<Value>> {
    let mut variants = vec![(String::new(), "Liga".to_owned(), 0, 0)];
    for (index, roman) in tier.divisions.iter().enumerate() {
        variants.push((
            format!("-{}", roman.to_lowercase()),
            format!("Division {roman}"),
            index + 1,
            0,
        ));
    }
    if tier.name == "Champion" && family == "player" {
        for place in [3, 2, 1] {
            variants.push((format!("-{place}"), format!("Platz {place}"), 0, place));
        }
    }
    variants.into_iter().map(|(suffix,label,division,place)| {
        let path = folder.join(family).join(format!("{}{suffix}.png",tier.slug));
        let data = STANDARD.encode(fs::read(path)?);
        Ok(json!({"label":label,"division":division,"place":place,"image":format!("data:image/png;base64,{data}")}))
    }).collect()
}

pub(super) fn gallery(root: &Path, ladder_path: &Path, output: &Path) -> Result<()> {
    let [vertex, fragment] = stages()?;
    let folder = root.join("art/baylee/ranks");
    let ladder = fs::read_to_string(ladder_path)?;
    let mut materials = Vec::new();
    for (tier, (description, seconds)) in tiers(&ladder)?.iter().zip(MATERIAL) {
        let mut material = json!({"name":tier.name,"description":description,"period":seconds});
        for family in FAMILIES {
            material[family] = variants(&folder, tier, family)?.into();
        }
        materials.push(material);
    }
    let web = Path::new(env!("CARGO_MANIFEST_DIR")).join("preview");
    let bundle = Command::new("deno")
        .current_dir(&web)
        .args([
            "bundle",
            "--quiet",
            "--platform",
            "browser",
            "--no-remote",
            "--no-npm",
            "gallery.ts",
        ])
        .output()?;
    if !bundle.status.success() {
        return Err(String::from_utf8_lossy(&bundle.stderr).into_owned().into());
    }
    let data = json!({"vertex":vertex,"fragment":fragment,"materials":materials});
    let html = include_str!("../preview/gallery.html")
        .replace("/*__APP__*/", &String::from_utf8(bundle.stdout)?)
        .replace("__CATALOG__", &data.to_string().replace('<', "\\u003c"));
    fs::write(output, html)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn rank_finish_validates_and_translates_to_webgl2() {
        for stage in super::stages().unwrap() {
            assert!(stage.starts_with("#version 300 es"));
        }
    }
}
