//! The phrase tables at runtime: each language's Fluent file, parsed once,
//! and every phrase rendered once per language for [`Table::text`].
//!
//! A message a language lacks reads in English (and the parity test refuses
//! the build that would ship it). Placeables are not wrapped in Unicode
//! isolation marks: the client draws left-to-right languages only, and the
//! marks would reach every label as two invisible characters.

use std::sync::OnceLock;

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource, FluentValue};

use super::Lang;

/// One table of phrases: its files, ids and argument names, loaded on first
/// use.
pub(crate) struct Table {
    /// The file stem (`interface`, `replay`, `chat`), for messages.
    name: &'static str,
    /// Each language's code and Fluent source.
    sources: &'static [(&'static str, &'static str)],
    /// The message ids, by variant.
    ids: &'static [&'static str],
    /// Each message's argument names, in the order they are passed.
    args: &'static [&'static [&'static str]],
    loaded: OnceLock<Loaded>,
}

struct Loaded {
    /// By [`Lang::ALL`]'s order; `None` for a language with no file.
    bundles: Vec<Option<FluentBundle<FluentResource>>>,
    /// Every phrase rendered with `{0}`, `{1}`, … as its arguments, by
    /// [`Lang::ALL`]'s order, then by variant.
    texts: Vec<Vec<&'static str>>,
}

impl Table {
    pub(crate) const fn new(
        name: &'static str,
        sources: &'static [(&'static str, &'static str)],
        ids: &'static [&'static str],
        args: &'static [&'static [&'static str]],
    ) -> Self {
        Self {
            name,
            sources,
            ids,
            args,
            loaded: OnceLock::new(),
        }
    }

    /// Each language's code and Fluent source.
    #[cfg(test)]
    pub(crate) const fn sources(&self) -> &'static [(&'static str, &'static str)] {
        self.sources
    }

    /// Each message's argument names, by variant.
    #[cfg(test)]
    pub(crate) const fn args(&self) -> &'static [&'static [&'static str]] {
        self.args
    }

    /// The message ids, by variant.
    #[cfg(test)]
    pub(crate) const fn ids(&self) -> &'static [&'static str] {
        self.ids
    }

    fn loaded(&self) -> &Loaded {
        self.loaded.get_or_init(|| {
            let bundles: Vec<Option<FluentBundle<FluentResource>>> = Lang::ALL
                .iter()
                .map(|lang| {
                    let (_, source) = self.sources.iter().find(|(code, _)| *code == lang.code())?;
                    Some(bundle(*lang, source))
                })
                .collect();
            let mut loaded = Loaded {
                bundles,
                texts: Vec::new(),
            };
            for lang in Lang::ALL {
                let texts = (0..self.ids.len())
                    .map(|index| {
                        let slots: Vec<String> = (0..self.args[index].len())
                            .map(|i| format!("{{{i}}}"))
                            .collect();
                        let slots: Vec<&str> = slots.iter().map(String::as_str).collect();
                        let text = self.render(&loaded, index, lang, &slots, None);
                        &*Box::leak(text.into_boxed_str())
                    })
                    .collect();
                loaded.texts.push(texts);
            }
            loaded
        })
    }

    /// Phrase `index` in `lang`, its arguments as `{0}`, `{1}`, ….
    pub(crate) fn text(&self, index: usize, lang: Lang) -> &'static str {
        self.loaded().texts[lang_index(lang)][index]
    }

    /// Phrase `index` in `lang` with `args` in their order; a missing one is
    /// left as `{N}`, so a forgotten argument is seen rather than swallowed.
    pub(crate) fn fill(&self, index: usize, lang: Lang, args: &[&str]) -> String {
        self.render(self.loaded(), index, lang, args, None)
    }

    /// As [`Self::fill`], with the argument named `count` set to `count`
    /// as a number, which is what picks a plural form.
    pub(crate) fn count(&self, index: usize, lang: Lang, count: usize, args: &[&str]) -> String {
        self.render(self.loaded(), index, lang, args, Some(count))
    }

    fn render(
        &self,
        loaded: &Loaded,
        index: usize,
        lang: Lang,
        args: &[&str],
        count: Option<usize>,
    ) -> String {
        let id = self.ids[index];
        let names = self.args[index];
        let mut fluent = FluentArgs::new();
        for (i, name) in names.iter().enumerate() {
            let value = match (count, *name) {
                (Some(n), "count") => FluentValue::from(n),
                _ => args.get(i).map_or_else(
                    || FluentValue::from(format!("{{{i}}}")),
                    |a| FluentValue::from(*a),
                ),
            };
            fluent.set(*name, value);
        }
        let english = lang_index(Lang::En);
        let found = [lang_index(lang), english].into_iter().find_map(|at| {
            let bundle = loaded.bundles[at].as_ref()?;
            let pattern = bundle.get_message(id)?.value()?;
            Some((bundle, pattern))
        });
        let Some((bundle, pattern)) = found else {
            return format!("{}::{id}", self.name);
        };
        let mut errors = Vec::new();
        bundle
            .format_pattern(pattern, Some(&fluent), &mut errors)
            .into_owned()
    }
}

fn lang_index(lang: Lang) -> usize {
    Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0)
}

/// A language's bundle of one table.
fn bundle(lang: Lang, source: &'static str) -> FluentBundle<FluentResource> {
    let id: unic_langid::LanguageIdentifier = lang.code().parse().unwrap_or_default();
    let mut bundle = FluentBundle::new_concurrent(vec![id]);
    bundle.set_use_isolating(false);
    // A file that does not parse is refused by the parity test; at runtime
    // its messages fall back to English rather than taking the client down.
    let resource =
        FluentResource::try_new(source.to_owned()).unwrap_or_else(|(partial, _)| partial);
    // A repeated id keeps its first message; the parity test refuses it.
    let _ = bundle.add_resource(resource);
    bundle
}
