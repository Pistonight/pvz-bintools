// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Pistonight/pvz-bintools contributors

use std::borrow::Cow;

use cu::pre::*;
use itertools::Itertools as _;
use roxmltree::{Document, Node};

use crate::tool::reanimc::data::{
    ReanimDefinition, ReanimTrack, ReanimTransform, ReanimTransformStrings, ReanimTransformValues,
    XmlFloat,
};

pub fn format_document(input: &[u8]) -> cu::Result<String> {
    let s = cu::check!(str::from_utf8(input), "input .reanim file must be ascii")?;
    // add a root node since the reanim file doesn't have one
    Ok(format!("<r>{s}</r>"))
}

pub struct ReanimDocument<'a> {
    root: Document<'a>,
}
impl<'a> ReanimDocument<'a> {
    pub fn parse_xml(s: &'a str) -> cu::Result<Self> {
        let document = cu::check!(Document::parse(s), "input is not valid xml")?;
        Ok(Self { root: document })
    }
}
impl ReanimDocument<'_> {
    #[cu::context("failed to parse reanim xml")]
    pub fn parse<'s>(&'s self) -> cu::Result<ReanimDefinition<'s, 's>> {
        let mut fps = None;
        let mut tracks = Vec::new();

        let mut seen_root = false;
        let mut frame_count = 0;
        for node in self.root.root().children() {
            let name = node.tag_name().name();
            match name {
                "r" => {
                    if seen_root {
                        cu::bail!("unexpected: there should only be one root");
                    }
                    seen_root = true;
                    for (i, node) in node.children().enumerate() {
                        let name = node.tag_name().name();
                        match name {
                            "fps" => {
                                let text = cu::check!(
                                    node.text(),
                                    "at root child {i}, expect <fps> to have a text child"
                                )?;
                                if fps.is_some() {
                                    cu::bail!("at root child {i}, duplicated <fps> node");
                                }
                                fps = Some(cu::check!(
                                    XmlFloat::parse(text),
                                    "at root child {i}, failed to parse <fps> tag into float"
                                )?)
                            }
                            "track" => {
                                let track = cu::check!(
                                    parse_track_node(node, frame_count),
                                    "at root child {i}, failed to parse <track> tag"
                                )?;
                                if frame_count == 0 {
                                    frame_count = track.transforms.len();
                                }
                                tracks.push(track);
                            }
                            "" => {
                                // ignore text node that are empty
                                let text = node.text().unwrap_or_default().trim();
                                cu::ensure!(text.is_empty(), "at root child {i}, {text:?}")?;
                            }
                            other => {
                                cu::bail!(
                                    "at root child {i}, expect <fps> or <track>, got <{other}>"
                                )
                            }
                        }
                    }
                }
                other => {
                    cu::bail!("unexpected tag at root: <{other}>")
                }
            }
        }

        let fps = cu::check!(fps, "missing <fps> tag in input")?;

        Ok(ReanimDefinition::new(fps.value, tracks))
    }
}

fn parse_track_node<'b>(node: Node<'b, 'b>, frame_count: usize) -> cu::Result<ReanimTrack<'b, 'b>> {
    let mut track_name = None;
    // all tracks should have the same number of frames
    let mut transforms = Vec::with_capacity(frame_count);
    for (i, node) in node.children().enumerate() {
        let name = node.tag_name().name();
        match name {
            "name" => {
                let text = cu::check!(
                    node.text(),
                    "at track node {i}, expect <name> to have a text child"
                )?;
                if track_name.is_some() {
                    cu::bail!("at track node {i}, duplicated <name> node");
                }
                cu::trace!("parsing track '{text}'");
                track_name = Some(text);
            }
            "t" => {
                let transform = cu::check!(
                    parse_transform_node(node),
                    "at track node {i}, failed to parse <t> tag"
                )?;
                transforms.push(transform);
            }
            "" => {
                // ignore text node that are empty
                let text = node.text().unwrap_or_default().trim();
                cu::ensure!(text.is_empty(), "at track node {i}, {text:?}")?;
            }
            other => {
                cu::bail!("at track node {i}, expect <name> or <t>, got <{other}>")
            }
        }
    }

    let name = cu::check!(track_name, "missing <name> tag in track")?;
    cu::trace!("done parsing track '{name}'");
    Ok(ReanimTrack::new(name, transforms))
}

fn parse_transform_node<'b>(node: Node<'b, 'b>) -> cu::Result<ReanimTransform<'b, 'b>> {
    let mut values: ReanimTransformValues<'_> = Default::default();
    let mut strings: ReanimTransformStrings<'_> = Default::default();

    for (i, node) in node.children().enumerate() {
        let name = node.tag_name().name();
        if name.is_empty() {
            // ignore text node that are empty
            let text = node.text().unwrap_or_default().trim();
            cu::ensure!(text.is_empty(), "at track node {i}, {text:?}")?;
            continue;
        }
        let text = node.text().unwrap_or_default();
        match name {
            "x" => {
                values.x = cu::check!(
                    XmlFloat::parse(text),
                    "at transform node {i}, failed to parse <x> tag"
                )?
            }
            "y" => {
                values.y = cu::check!(
                    XmlFloat::parse(text),
                    "at transform node {i}, failed to parse <y> tag"
                )?
            }
            "kx" => {
                values.kx = cu::check!(
                    XmlFloat::parse(text),
                    "at transform node {i}, failed to parse <kx> tag"
                )?
            }
            "ky" => {
                values.ky = cu::check!(
                    XmlFloat::parse(text),
                    "at transform node {i}, failed to parse <ky> tag"
                )?
            }
            "sx" => {
                values.sx = cu::check!(
                    XmlFloat::parse(text),
                    "at transform node {i}, failed to parse <sx> tag"
                )?
            }
            "sy" => {
                values.sy = cu::check!(
                    XmlFloat::parse(text),
                    "at transform node {i}, failed to parse <sy> tag"
                )?
            }
            "f" => {
                values.frame = cu::check!(
                    XmlFloat::parse(text),
                    "at transform node {i}, failed to parse <f> tag"
                )?
            }
            "a" => {
                values.alpha = cu::check!(
                    XmlFloat::parse(text),
                    "at transform node {i}, failed to parse <a> tag"
                )?
            }
            "i" => strings.image = parse_text(text),
            "font" => strings.font = parse_text(text),
            "text" => strings.text = parse_text(text),
            other => {
                cu::bail!("at transform node {i}, invalid transform child tag <{other}>")
            }
        }
    }

    Ok(ReanimTransform { values, strings })
}

fn parse_text(s: &str) -> Cow<'_, str> {
    let s = s.trim();
    if !s.contains("  ") {
        return s.into();
    }
    s.split_whitespace().join(" ").into()
}
