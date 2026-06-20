// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Pistonight/pvz-bintools contributors

use std::fmt;

use cu::pre::*;

/// A pattern that supports deserialization.
///
/// The pattern must be prefixed with `regex:` or `glob:` to select the kind.
pub struct Pattern {
    /// The literal pattern string, without the `regex:`/`glob:` prefix.
    literal: String,
    kind: PatternKind,
}

enum PatternKind {
    Regex(regex::Regex),
    Glob(glob::Pattern),
}

impl Pattern {
    /// Check if a relative path (without the `./` prefix) matches the pattern.
    pub fn matches(&self, path: &str) -> bool {
        match &self.kind {
            PatternKind::Regex(regex) => regex.is_match(path),
            PatternKind::Glob(glob) => glob.matches(path),
        }
    }
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = match &self.kind {
            PatternKind::Regex(_) => "regex",
            PatternKind::Glob(_) => "glob",
        };
        write!(f, "{prefix}:{}", self.literal)
    }
}

impl<'de> Deserialize<'de> for Pattern {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let pattern = String::deserialize(deserializer)?;
        if let Some(rest) = pattern.strip_prefix("regex:") {
            let regex = regex::Regex::new(rest).map_err(serde::de::Error::custom)?;
            Ok(Pattern {
                literal: rest.to_owned(),
                kind: PatternKind::Regex(regex),
            })
        } else if let Some(rest) = pattern.strip_prefix("glob:") {
            let glob = glob::Pattern::new(rest).map_err(serde::de::Error::custom)?;
            Ok(Pattern {
                literal: rest.to_owned(),
                kind: PatternKind::Glob(glob),
            })
        } else {
            Err(serde::de::Error::custom(
                "pattern must be prefixed with `regex:` or `glob:`",
            ))
        }
    }
}
