//! Reading `croft.toml` with the `toml` crate, keeping where each name was written.

use std::collections::BTreeMap;

use croft_syntax::ast::Identifier;
use croft_syntax::{Diagnostic, SourceId};
use serde::Deserialize;
use toml::Spanned;

use super::{span_at, Manifest, TargetConfig};

/// `croft.toml` as written, before it is turned into a `Manifest`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawManifest {
    package: RawPackage,
    #[serde(default)]
    target: BTreeMap<Spanned<String>, RawTarget>,
}

/// The `[package]` table.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPackage {
    name: Spanned<String>,
    version: Option<String>,
}

/// One `[target.<name>]` table, as written.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTarget {
    main: Option<Spanned<String>>,
    #[serde(default)]
    native: Vec<Spanned<String>>,
    #[serde(default)]
    adapters: BTreeMap<Spanned<String>, Spanned<String>>,
    checks: Option<bool>,
}

/// Reads `croft.toml` into a `Manifest`, keeping where each name was written.
pub fn parse(text: &str, source: SourceId) -> Result<Manifest, Diagnostic> {
    let raw: RawManifest = toml::from_str(text).map_err(|error| {
        let span = span_at(text, error.span().map_or(0, |range| range.start), source);
        Diagnostic::new(error.message().trim().to_string(), span)
    })?;

    let name = |value: &Spanned<String>| {
        // A string's span starts at its opening quote; point at the text inside.
        let offset = value.span().start;

        let inside = if text[offset..].starts_with('"') {
            offset + 1
        } else {
            offset
        };

        Identifier {
            name: value.get_ref().clone(),
            span: span_at(text, inside, source),
        }
    };

    let targets = raw.target.iter().map(|(target_name, target)| TargetConfig {
        name: name(target_name),
        main: target.main.as_ref().map(name),
        native: target.native.iter().map(name).collect(),
        adapters: target
            .adapters
            .iter()
            .map(|(port, adapter)| (name(port), name(adapter)))
            .collect(),
        checks: target.checks.unwrap_or(true),
    });

    Ok(Manifest {
        name: name(&raw.package.name),
        version: raw.package.version,
        targets: targets.collect(),
    })
}
