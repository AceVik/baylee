//! Generate exhaustive, typed visitors for the public card DSL. This reads
//! Rust declarations, never the external card-script corpus or Debug strings.
use sha2::{Digest, Sha256};
use std::{env, fmt::Write as _, fs, path::Path};
use syn::{Fields, Item, Visibility};

fn fields_code(fields: &Fields, label: &str, target: &str, is_enum: bool) -> (String, String) {
    let mut pattern = Vec::new();
    let mut code = String::from("let children = vec![\n");
    for (i, f) in fields.iter().enumerate() {
        let field = f
            .ident
            .as_ref()
            .map_or_else(|| i.to_string(), ToString::to_string);
        let binding = format!("f{i}");
        pattern.push(if f.ident.is_some() {
            format!("{field}: {binding}")
        } else {
            binding.clone()
        });
        // Names participate in rules and are retained. Effective abilities/keywords
        // are supplied by CardDef's face accessors at the root instead.
        if (label == "FaceDef" && ["abilities", "keywords"].contains(&field.as_str()))
            || (label == "TokenDef" && ["scryfall_id"].contains(&field.as_str()))
        {
            continue;
        }
        let access = if is_enum {
            binding
        } else {
            format!("&self.{field}")
        };
        writeln!(code, "b.field(\"{label}.{field}\", {access}),").unwrap();
    }
    code.push_str("];\n");
    write!(code, "b.node(\"{label}\", &[], children)").unwrap();
    let pat = match fields {
        Fields::Named(_) => format!("{target} {{ {} }}", pattern.join(", ")),
        Fields::Unnamed(_) => format!("{target}({})", pattern.join(", ")),
        Fields::Unit => target.to_string(),
    };
    (pat, code)
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if env::var_os("CARGO_FEATURE_PLAY").is_none() {
        return;
    }
    let root = Path::new("../baylee-cards-dsl/src");
    let files = [
        ("lib.rs", ""),
        ("ability.rs", "ability::"),
        ("cost.rs", "cost::"),
        ("filter.rs", "filter::"),
        ("static_ability.rs", "static_ability::"),
        ("special.rs", "special::"),
        ("text.rs", "text::"),
        ("effect/mod.rs", "effect::"),
        ("effect/amount.rs", "effect::"),
        ("effect/counters.rs", "effect::"),
        ("effect/division.rs", "effect::"),
        ("effect/mana.rs", "effect::"),
        ("effect/play.rs", "effect::"),
        ("effect/search.rs", "effect::"),
        ("effect/targets.rs", "effect::"),
    ];
    let mut out = String::new();
    let mut hash = Sha256::new();
    for (file, module) in files {
        let path = root.join(file);
        println!("cargo:rerun-if-changed={}", path.display());
        let source = fs::read_to_string(&path).expect("read card DSL");
        hash.update(file.as_bytes());
        hash.update(source.as_bytes());
        for item in syn::parse_file(&source).expect("parse card DSL").items {
            let (name, body) = match item {
                Item::Enum(e) if matches!(e.vis, Visibility::Public(_)) => {
                    let name = e.ident.to_string();
                    if name == "Coverage" {
                        continue;
                    }
                    let mut arms = String::new();
                    for v in &e.variants {
                        let label = format!("{name}::{}", v.ident);
                        let (pat, code) =
                            fields_code(&v.fields, &label, &format!("Self::{}", v.ident), true);
                        writeln!(arms, "{pat} => {{ {code} }},").unwrap();
                    }
                    (name, format!("match self {{ {arms} }}"))
                }
                Item::Struct(s) if matches!(s.vis, Visibility::Public(_)) => {
                    let name = s.ident.to_string();
                    if ["CardDef", "KeywordSet"].contains(&name.as_str()) {
                        continue;
                    }
                    (name.clone(), fields_code(&s.fields, &name, "", false).1)
                }
                _ => continue,
            };
            write!(out, "impl TreeEncode for baylee_cards_dsl::{module}{name} {{\n#[allow(clippy::too_many_lines, reason = \"Exhaustive generated typed DSL visitor\")]\nfn tree(&self, b: &mut Forest) -> u32 {{ b.enter(); let result = {{ {body} }}; b.leave(); result }}\n}}\n").unwrap();
        }
    }
    let mut digest = String::new();
    for byte in hash.finalize() {
        write!(digest, "{byte:02x}").unwrap();
    }
    writeln!(out, "pub const DSL_SOURCE_SHA256: &str = \"{digest}\";").unwrap();
    fs::write(
        Path::new(&env::var("OUT_DIR").unwrap()).join("card_tree.rs"),
        out,
    )
    .unwrap();
}
