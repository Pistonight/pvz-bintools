// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Pistonight/pvz-bintools contributors

use std::collections::BTreeSet;
use std::path::Path;

use cu::pre::*;

use crate::tool::reanimc::data::{ReanimDefinition, ReanimTrack, ReanimTransform};
use crate::tool::resc::{Manifest, ManifestItemTag};

#[derive(Default)]
pub struct Checker {
    loaded: bool,
    image_ids: BTreeSet<String>,
    font_ids: BTreeSet<String>,
}

impl Checker {
    pub fn load_from_manifest(&mut self, manifest: &Manifest) {
        for group in &manifest.resource_groups {
            for item_ref in group.iter() {
                match item_ref.item.tag {
                    ManifestItemTag::Image => {
                        self.image_ids.insert(item_ref.full_id);
                    }
                    ManifestItemTag::Font => {
                        self.font_ids.insert(item_ref.full_id);
                    }
                    _ => {
                        // ignore
                    }
                }
            }
        }
        self.loaded = true;
    }

    pub fn load_from_pak_dir(&mut self, pak_dir: &Path) -> cu::Result<()> {
        let defs = [
            ("IMAGE_", pak_dir.join("particles")),
            ("IMAGE_REANIM_", pak_dir.join("reanim")),
            ("IMAGE_REANIM_", pak_dir.join("images")),
        ];
        for (prefix, path) in defs {
            if !path.exists() {
                continue;
            }
            for entry in cu::fs::read_dir(path)? {
                let entry = entry?;
                let mut name = entry.file_name().into_utf8()?;
                name.make_ascii_uppercase();
                for suffix in [".PNG", ".JPG", ".GIF"] {
                    if let Some(name) = name.strip_suffix(suffix) {
                        self.image_ids.insert(format!("{prefix}{name}"));
                    }
                }
            }
        }

        self.loaded = true;
        Ok(())
    }

    pub fn check_definition(&self, definition: &ReanimDefinition<'_, '_>) -> cu::Result<()> {
        if !self.loaded {
            return Ok(());
        }
        for t in &definition.tracks {
            cu::check!(self.check_track(t), "track '{}' is invalid", t.name)?;
        }
        Ok(())
    }

    fn check_track(&self, track: &ReanimTrack<'_, '_>) -> cu::Result<()> {
        for (i, transform) in track.transforms.iter().enumerate() {
            cu::check!(
                self.check_transform(transform),
                "track transforms[{i}] is invalid"
            )?;
        }
        Ok(())
    }

    fn check_transform(&self, transform: &ReanimTransform<'_, '_>) -> cu::Result<()> {
        let image = transform.strings.image.as_ref();
        if !image.is_empty() && !self.image_ids.contains(image) {
            cu::bail!("transform references non-existent image '{image}'");
        }
        let font = transform.strings.font.as_ref();
        if !font.is_empty() && !self.font_ids.contains(font) {
            cu::bail!("transform references non-existent font '{font}'");
        }
        Ok(())
    }
}
