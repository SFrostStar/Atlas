//! Native registry adapter and ModSearch's weighted, stable ordering.
use super::config::{Config, Sorting};
use crate::{lang, mod_list::ModList};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct Details {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub authors: Vec<String>,
    pub licenses: Vec<String>,
    pub homepage: String,
    pub issues: String,
    pub source: String,
    pub credits: BTreeMap<String, Vec<String>>,
    pub links: BTreeMap<String, String>,
    pub dependencies: BTreeMap<String, String>,
    pub icon: String,
    pub parent: Option<String>,
    pub library: bool,
    pub client: bool,
    pub deprecated: bool,
    pub hidden: bool,
    /// Only the native registry can provide an Atlas-compatible update URL.
    pub update_url: Option<String>,
}
#[derive(Clone, Debug)]
pub struct Entry {
    pub details: Details,
    pub installed_index: Option<usize>,
    pub enabled: bool,
    pub configurable: bool,
    pub problem: Option<String>,
}
impl Entry {
    pub fn name(&self, c: &Config) -> String {
        if c.translate_names {
            lang::get_or(
                &format!("modmenu.nameTranslation.{}", self.details.id),
                &self.details.name,
            )
        } else {
            self.details.name.clone()
        }
    }
    pub fn description(&self, c: &Config) -> String {
        if c.translate_descriptions {
            lang::get_or(
                &format!("modmenu.descriptionTranslation.{}", self.details.id),
                &self.details.description,
            )
        } else {
            self.details.description.clone()
        }
    }
    pub fn summary(&self, c: &Config) -> String {
        self.description(c)
            .lines()
            .next()
            .unwrap_or_default()
            .to_owned()
    }
    pub fn has_config(&self, c: &Config) -> bool {
        self.configurable && !c.hidden_configs.contains(&self.details.id)
    }
    pub fn hidden(&self, c: &Config) -> bool {
        self.details.hidden || c.hidden_mods.contains(&self.details.id)
    }
    pub fn update(&self, _c: &Config) -> bool {
        self.details.update_url.is_some()
    }
}
pub fn entries(list: &ModList) -> Vec<Entry> {
    static PORTS: std::sync::OnceLock<Vec<Details>> = std::sync::OnceLock::new();
    let ports = PORTS.get_or_init(|| {
        serde_json::from_str(include_str!("ports.json")).expect("bundled native metadata")
    });
    let mut out = Vec::new();
    out.push(Entry {
        details: serde_json::from_str(include_str!("modmenu.json"))
            .expect("bundled Mod Menu metadata"),
        installed_index: None,
        enabled: true,
        configurable: true,
        problem: None,
    });
    out.push(Entry {
        details: Details {
            id: "atlas".into(),
            name: "Atlas".into(),
            version: option_env!("ATLAS_VERSION").unwrap_or("1.1.0").into(),
            description: "Minecraft on Rust, based on Horizon.".into(),
            authors: vec!["SFrostStar".into()],
            licenses: vec!["MIT".into()],
            homepage: "https://sfroststar.github.io/Atlas/".into(),
            issues: "https://github.com/SFrostStar/Atlas/issues".into(),
            source: "https://github.com/SFrostStar/Atlas".into(),
            icon: "atlas/loading-logo".into(),
            ..Default::default()
        },
        installed_index: None,
        enabled: true,
        configurable: false,
        problem: None,
    });
    for (index, installed) in list.installed.iter().enumerate() {
        let details = list
            .details
            .get(&installed.id)
            .cloned()
            .or_else(|| ports.iter().find(|d| d.id == installed.id).cloned())
            .unwrap_or_else(|| {
                let (name, version) = installed
                    .title
                    .rsplit_once(' ')
                    .filter(|(_, v)| v.starts_with(|c: char| c.is_ascii_digit()))
                    .map_or((installed.title.as_str(), ""), |(n, v)| (n, v));
                Details {
                    id: installed.id.clone(),
                    name: name.into(),
                    version: version.into(),
                    description: installed.description.clone(),
                    client: installed.native.is_some(),
                    ..Default::default()
                }
            });
        let configurable = installed
            .native
            .as_ref()
            .is_some_and(|p| !p.keys.is_empty() || !p.buttons.is_empty());
        out.push(Entry {
            details,
            installed_index: Some(index),
            enabled: list.is_on(index),
            configurable,
            problem: installed.problem.clone(),
        });
    }
    out
}
fn score(entry: &Entry, all: &[Entry], query: &str, c: &Config, depth: usize) -> u8 {
    if depth > 32
        || entry.hidden(c)
        || c.show_libraries
            .hide(entry.details.library, entry.has_config(c))
    {
        return 0;
    }
    let d = &entry.details;
    if [&d.name, &entry.name(c), &d.id]
        .iter()
        .any(|s| s.to_lowercase().contains(query))
    {
        return if query.encode_utf16().count() >= 3 {
            2
        } else {
            1
        };
    }
    if entry.description(c).to_lowercase().contains(query)
        || entry.summary(c).to_lowercase().contains(query)
        || d.authors.iter().any(|a| a.to_lowercase().contains(query))
    {
        return 1;
    }
    for (key, yes) in [
        ("library", d.library),
        ("deprecated", d.deprecated),
        ("clientside", d.client),
        ("configurable", entry.has_config(c)),
        ("hasUpdate", entry.update(c)),
    ] {
        if yes
            && lang::get_or(&format!("modmenu.searchTerms.{key}"), key)
                .to_lowercase()
                .contains(query)
        {
            return 1;
        }
    }
    all.iter()
        .filter(|child| child.details.parent.as_deref() == Some(&d.id))
        .map(|child| score(child, all, query, c, depth + 1))
        .find(|value| *value > 0)
        .unwrap_or(0)
}
fn child_update(entry: &Entry, all: &[Entry], c: &Config, depth: usize) -> bool {
    entry.update(c)
        || (depth < 32
            && all
                .iter()
                .filter(|child| child.details.parent.as_deref() == Some(&entry.details.id))
                .any(|child| child_update(child, all, c, depth + 1)))
}
pub fn filtered(all: &[Entry], c: &Config, query: &str) -> Vec<usize> {
    let mut rows: Vec<_> = all
        .iter()
        .enumerate()
        .filter(|(_, e)| {
            (if c.config_mode {
                e.has_config(c)
            } else {
                !e.hidden(c)
            }) && !c.show_libraries.hide(e.details.library, e.has_config(c))
        })
        .map(|(i, _)| i)
        .collect();
    match c.sorting {
        Sorting::Ascending => rows.sort_by_cached_key(|&i| {
            all[i]
                .name(c)
                .to_lowercase()
                .encode_utf16()
                .collect::<Vec<_>>()
        }),
        Sorting::Descending => rows.sort_by_cached_key(|&i| {
            std::cmp::Reverse(
                all[i]
                    .name(c)
                    .to_lowercase()
                    .encode_utf16()
                    .collect::<Vec<_>>(),
            )
        }),
        Sorting::HasUpdate => {
            rows.sort_by_key(|&i| std::cmp::Reverse(child_update(&all[i], all, c, 0)))
        }
    }
    if !query.is_empty() {
        let q = query.to_lowercase();
        rows.retain(|&i| score(&all[i], all, &q, c, 0) > 0);
        rows.sort_by_key(|&i| std::cmp::Reverse(score(&all[i], all, &q, c, 0)));
    }
    rows
}
pub fn count(all: &[Entry], c: &Config) -> usize {
    all.iter()
        .filter(|e| {
            (c.count_libraries || !e.details.library)
                && (c.count_children || e.details.parent.is_none())
                && (c.count_hidden_mods || !e.hidden(c))
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn e(id: &str, name: &str, description: &str) -> Entry {
        Entry {
            details: Details {
                id: id.into(),
                name: name.into(),
                description: description.into(),
                ..Default::default()
            },
            installed_index: None,
            enabled: true,
            configurable: false,
            problem: None,
        }
    }
    #[test]
    fn name_matches_outrank_description_matches_but_short_queries_preserve_sort_order() {
        let all = vec![e("a", "A mod", "zoom"), e("zoom", "Zoom", "different")];
        let c = Config::default();
        assert_eq!(filtered(&all, &c, "zoom"), [1, 0]);
        assert_eq!(filtered(&all, &c, "zo"), [0, 1]);
    }
    #[test]
    fn libraries_with_configuration_and_hidden_entries_follow_reference_filters() {
        let mut all = vec![e("lib", "Library", ""), e("hidden", "Hidden", "")];
        all[0].details.library = true;
        let mut c = Config::default();
        c.hidden_mods.insert("hidden".into());
        assert!(filtered(&all, &c, "").is_empty());
        all[0].configurable = true;
        assert_eq!(filtered(&all, &c, ""), [0]);
        c.show_libraries = super::super::config::Libraries::False;
        assert!(filtered(&all, &c, "").is_empty());
    }
}
