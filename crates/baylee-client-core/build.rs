//! Generates the interface's phrase enums from the English Fluent files
//! (`locales/en/*.ftl`, `docs/client.md` §"The interface's own words").
//!
//! Each file is one table: its `###` comment is the enum's doc, each message
//! a variant named by its id with its `#` comment as the variant's doc, and
//! a `# Arguments: a, b` line the order a caller passes the arguments in.
//! The variants, their ids and argument names are written to
//! `$OUT_DIR/phrases.rs`; every language's file is embedded beside them.
//! Only the English files make variants: a message another language has
//! and English has not is refused by the parity test, never generated.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use fluent_syntax::ast::{self, Entry};

/// The tables: file stem, enum name.
const TABLES: [(&str, &str); 4] = [
    ("interface", "Phrase"),
    ("replay", "ReplayPhrase"),
    ("chat", "ChatPhrase"),
    ("server", "ServerPhrase"),
];

fn main() {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets it"));
    let locales = root.join("locales");
    println!("cargo::rerun-if-changed={}", locales.display());
    let mut languages: Vec<String> = std::fs::read_dir(&locales)
        .expect("locales/")
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    languages.sort();
    let mut out = String::new();
    for (stem, name) in TABLES {
        let english = locales.join("en").join(format!("{stem}.ftl"));
        println!("cargo::rerun-if-changed={}", english.display());
        table(&mut out, &locales, &languages, stem, name, &english);
    }
    let dest = PathBuf::from(std::env::var("OUT_DIR").expect("cargo sets it")).join("phrases.rs");
    std::fs::write(dest, out).expect("writing phrases.rs");
}

/// One table's English messages: id, doc lines, argument names.
struct Message {
    id: String,
    doc: Vec<String>,
    args: Vec<String>,
}

/// One enum, its constants and its table.
fn table(
    out: &mut String,
    locales: &Path,
    languages: &[String],
    stem: &str,
    name: &str,
    english: &Path,
) {
    let (doc, messages) = read(english);
    for line in &doc {
        let _ = writeln!(out, "/// {line}");
    }
    let _ = writeln!(out, "#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]");
    let _ = writeln!(out, "pub enum {name} {{");
    for message in &messages {
        for line in &message.doc {
            let _ = writeln!(out, "    /// {line}");
        }
        let _ = writeln!(out, "    {},", message.id);
    }
    let _ = writeln!(out, "}}\n");
    let (mut all, mut ids, mut args) = (String::new(), String::new(), String::new());
    for message in &messages {
        let _ = writeln!(all, "        Self::{},", message.id);
        let _ = writeln!(ids, "        {:?},", message.id);
        let _ = writeln!(args, "        &{:?},", message.args);
    }
    let table = format!("{}_TABLE", stem.to_ascii_uppercase());
    let _ = write!(
        out,
        r#"impl {name} {{
    /// Every phrase, in the file's order.
    pub const ALL: &'static [Self] = &[
{all}    ];

    /// The message ids, by variant.
    #[allow(clippy::large_stack_arrays, reason = "a constant in static memory, never on the stack")]
    pub(crate) const IDS: &'static [&'static str] = &[
{ids}    ];

    /// Each phrase's argument names, in the order they are passed.
    #[allow(clippy::large_stack_arrays, reason = "a constant in static memory, never on the stack")]
    pub(crate) const ARGS: &'static [&'static [&'static str]] = &[
{args}    ];

    /// This phrase in one language, its arguments as `{{0}}`, `{{1}}`, ….
    #[must_use]
    pub fn text(self, lang: Lang) -> &'static str {{
        {table}.text(self as usize, lang)
    }}

    /// This phrase with its arguments, in the order the `# Arguments`
    /// line of the English file names them; one missing is left as `{{N}}`.
    #[must_use]
    pub fn fill(self, lang: Lang, args: &[&str]) -> String {{
        {table}.fill(self as usize, lang, args)
    }}

    /// As [`Self::text`], with the argument named `count` set to `count`:
    /// the form a counted phrase takes for that number, every other
    /// argument as `{{0}}`, `{{1}}`, ….
    #[must_use]
    pub fn text_count(self, lang: Lang, count: usize) -> String {{
        {table}.count_text(self as usize, lang, count)
    }}

    /// As [`Self::fill`], with the argument named `count` set to `count`
    /// as a number: what picks the language's plural form.
    #[must_use]
    pub fn fill_count(self, lang: Lang, count: usize, args: &[&str]) -> String {{
        {table}.count(self as usize, lang, count, args)
    }}
}}

pub(crate) static {table}: crate::i18n::fluent::Table = crate::i18n::fluent::Table::new(
    {stem:?},
    &[
"#
    );
    for language in languages {
        let file = locales.join(language).join(format!("{stem}.ftl"));
        if file.is_file() {
            println!("cargo::rerun-if-changed={}", file.display());
            let _ = writeln!(
                out,
                "        ({language:?}, include_str!({:?})),",
                file.display().to_string()
            );
        }
    }
    let _ = writeln!(out, "    ],\n    {name}::IDS,\n    {name}::ARGS,\n);\n");
}

/// The file's `###` doc and its messages.
fn read(english: &Path) -> (Vec<String>, Vec<Message>) {
    let source = std::fs::read_to_string(english).expect("an English file");
    let resource = fluent_syntax::parser::parse(source.as_str())
        .unwrap_or_else(|(_, errors)| panic!("{}: {errors:?}", english.display()));
    let mut doc = Vec::new();
    let mut messages = Vec::new();
    for entry in &resource.body {
        match entry {
            Entry::ResourceComment(comment) => {
                doc.extend(comment.content.iter().map(|l| (*l).to_owned()));
            }
            Entry::Message(message) => messages.push(message_of(message)),
            _ => {}
        }
    }
    (doc, messages)
}

fn message_of(message: &ast::Message<&str>) -> Message {
    let id = message.id.name.to_owned();
    let mut doc = Vec::new();
    let mut args = Vec::new();
    for line in message.comment.iter().flat_map(|c| c.content.iter()) {
        if let Some(list) = line.strip_prefix("Arguments: ") {
            args = list.split(", ").map(str::to_owned).collect();
        } else {
            doc.push((*line).to_owned());
        }
    }
    if doc.is_empty() {
        doc.push(format!("`{id}`."));
    }
    Message { id, doc, args }
}
