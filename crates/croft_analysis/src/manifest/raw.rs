//! Reading `croft.toml` with the `toml` crate, keeping where each name was written.

use std::collections::BTreeMap;

use croft_syntax::ast::Identifier;
use croft_syntax::{Diagnostic, SourceId};
use serde::Deserialize;
use toml::Spanned;

use super::{span_at, Manifest, TargetConfig, DEFAULT_BYTES_LENGTH};

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
    bytes_length: Option<Spanned<i64>>,
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

    let mut targets = Vec::new();

    for (target_name, target) in &raw.target {
        targets.push(TargetConfig {
            name: name(target_name),
            main: target.main.as_ref().map(name),
            native: target.native.iter().map(name).collect(),
            adapters: target
                .adapters
                .iter()
                .map(|(port, adapter)| (name(port), name(adapter)))
                .collect(),
            checks: target.checks.unwrap_or(true),
            bytes_length: bytes_length(text, source, target.bytes_length.as_ref())?,
        });
    }

    Ok(Manifest {
        name: name(&raw.package.name),
        version: raw.package.version,
        targets,
    })
}

/// A target's `bytes_length`: from 1 to 65535, as a `Bytes` counts its
/// values in 16 bits, or the default when it has none.
fn bytes_length(
    text: &str,
    source: SourceId,
    written: Option<&Spanned<i64>>,
) -> Result<u16, Diagnostic> {
    let Some(written) = written else {
        return Ok(DEFAULT_BYTES_LENGTH);
    };

    match u16::try_from(*written.get_ref()) {
        Ok(length) if length > 0 => Ok(length),
        _ => Err(Diagnostic::new(
            format!(
                "`bytes_length` is {}, but a `Bytes` holds from 1 to 65535 values",
                written.get_ref()
            ),
            span_at(text, written.span().start, source),
        )),
    }
}
