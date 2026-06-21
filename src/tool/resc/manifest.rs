// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Pistonight/pvz-bintools contributors

use std::collections::BTreeMap;
use std::fmt::Write as _;

use cu::pre::*;
use roxmltree::{Document, Node};

/// Structure for the manifest file
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Manifest {
    pub resource_groups: Vec<ManifestResourceGroup>,
}
impl Manifest {
    #[cu::context("failed to parse resource manifest xml")]
    pub fn try_parse_xml(xml: &str) -> cu::Result<Self> {
        let document = cu::check!(Document::parse(xml), "input is not valid xml")?;

        let mut resource_groups = Vec::new();
        let mut seen_root = false;
        for node in document.root().children() {
            if !node.is_element() {
                continue;
            }
            let name = node.tag_name().name();
            match name {
                "ResourceManifest" => {
                    if seen_root {
                        cu::bail!("unexpected: there should only be one <ResourceManifest>");
                    }
                    seen_root = true;
                    for node in node.children() {
                        if !node.is_element() {
                            continue;
                        }
                        let name = node.tag_name().name();
                        match name {
                            "Resources" => {
                                resource_groups.push(cu::check!(
                                    ManifestResourceGroup::parse(node),
                                    "failed to parse <Resources> tag"
                                )?);
                            }
                            other => {
                                cu::bail!("expect <Resources>, got <{other}>");
                            }
                        }
                    }
                }
                other => {
                    cu::bail!("unexpected tag at root: <{other}>, expect <ResourceManifest>");
                }
            }
        }

        cu::ensure!(seen_root, "missing <ResourceManifest> root element")?;

        Ok(Self { resource_groups })
    }

    /// Serialize the manifest to an XML string, including the `<?xml?>` header.
    pub fn to_xml(&self) -> String {
        let mut out = String::new();
        self.write_xml(&mut out);
        out
    }

    fn write_xml(&self, out: &mut String) {
        let _ = writeln!(out, r#"<?xml version="1.0"?>"#);
        let _ = writeln!(out, "<ResourceManifest>");
        for group in &self.resource_groups {
            // blank line separating each <Resources> group
            out.push('\n');
            group.write_xml(out);
        }
        let _ = writeln!(out, "\n</ResourceManifest>");
    }

    /// Sort the manifest to be comparable
    pub fn sort(&mut self) {
        for group in &mut self.resource_groups {
            group.sort()
        }
        // sort_by_key does not allow reference
        self.resource_groups.sort_by(|a, b| a.id.cmp(&b.id));
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ManifestResourceGroup {
    pub id: String,
    pub items_with_defaults: Vec<ManifestItemsWithDefault>,
}
impl ManifestResourceGroup {
    fn parse(node: Node) -> cu::Result<Self> {
        let id = cu::check!(
            node.attribute("id"),
            "missing `id` attribute on <Resources>"
        )?;

        // each <SetDefaults> starts a new segment; items before the first
        // <SetDefaults> form a leading segment with no defaults
        let mut set_defaults: Option<Node> = None;
        let mut item_nodes: Vec<Node> = Vec::new();
        let mut items_with_defaults = Vec::new();
        let mut started = false;

        for child in node.children() {
            if !child.is_element() {
                continue;
            }
            let name = child.tag_name().name();
            match name {
                "SetDefaults" => {
                    if started {
                        items_with_defaults.push(cu::check!(
                            ManifestItemsWithDefault::parse(set_defaults, &item_nodes),
                            "failed to parse resource segment"
                        )?);
                        item_nodes.clear();
                    }
                    set_defaults = Some(child);
                    started = true;
                }
                "Image" | "Font" | "Sound" => {
                    started = true;
                    item_nodes.push(child);
                }
                other => {
                    cu::bail!("expect <SetDefaults>, <Image>, <Font> or <Sound>, got <{other}>");
                }
            }
        }

        if started {
            items_with_defaults.push(cu::check!(
                ManifestItemsWithDefault::parse(set_defaults, &item_nodes),
                "failed to parse resource segment"
            )?);
        }

        Ok(Self {
            id: id.to_owned(),
            items_with_defaults,
        })
    }
    pub fn iter(&self) -> impl Iterator<Item = ManifestItemRef<'_>> {
        self.items_with_defaults.iter().flat_map(|x| x.iter())
    }

    fn write_xml(&self, out: &mut String) {
        let _ = writeln!(out, r#"<Resources id="{}">"#, self.id);
        for segment in &self.items_with_defaults {
            segment.write_xml(out);
            let _ = writeln!(out);
        }
        let _ = writeln!(out, "</Resources>");
    }

    pub fn is_empty(&self) -> bool {
        for items in &self.items_with_defaults {
            if !items.is_empty() {
                return false;
            }
        }
        true
    }

    pub fn len(&self) -> usize {
        self.items_with_defaults.iter().map(|x| x.len()).sum()
    }

    pub fn sort(&mut self) {
        for items in &mut self.items_with_defaults {
            items.sort();
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ManifestItemsWithDefault {
    pub set_defaults: Option<ManifestSetDefaults>,
    pub items: Vec<ManifestItem>,
}
impl ManifestItemsWithDefault {
    fn parse(set_defaults: Option<Node>, item_nodes: &[Node]) -> cu::Result<Self> {
        let set_defaults = match set_defaults {
            Some(node) => Some(cu::check!(
                ManifestSetDefaults::parse(node),
                "failed to parse <SetDefaults> tag"
            )?),
            None => None,
        };

        let mut items = Vec::with_capacity(item_nodes.len());
        for node in item_nodes {
            items.push(cu::check!(
                ManifestItem::parse(*node),
                "failed to parse resource item"
            )?);
        }

        Ok(Self {
            set_defaults,
            items,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = ManifestItemRef<'_>> {
        self.items.iter().map(|item| {
            match &self.set_defaults {
                None => ManifestItemRef {
                    full_id: item.id.clone(),
                    full_path: item.path.clone(),
                    item,
                },
                Some(defaults) => {
                    let full_id = format!("{}{}", defaults.idprefix, item.id);
                    let mut full_path = defaults.path.clone();
                    if !full_path.ends_with('/') {
                        full_path.push('/');
                    }
                    // FUTURE: trim_prefix is better
                    let item_path = item.path.strip_prefix('/').unwrap_or(&item.path);
                    full_path.push_str(item_path);

                    ManifestItemRef {
                        full_id,
                        full_path,
                        item,
                    }
                }
            }
        })
    }

    fn write_xml(&self, out: &mut String) {
        if let Some(defaults) = &self.set_defaults {
            defaults.write_xml(out);
        }
        for item in &self.items {
            item.write_xml(out);
        }
    }

    pub fn sort(&mut self) {
        // weird lifetime issue when trying to use sort_by_key
        self.items
            .sort_by(|a, b| (a.tag, &a.id).cmp(&(b.tag, &b.id)))
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize)]
pub struct ManifestSetDefaults {
    pub path: String,
    pub idprefix: String,
}
impl ManifestSetDefaults {
    fn parse(node: Node) -> cu::Result<Self> {
        let path = cu::check!(
            node.attribute("path"),
            "missing `path` attribute on <SetDefaults>"
        )?;
        let idprefix = cu::check!(
            node.attribute("idprefix"),
            "missing `idprefix` attribute on <SetDefaults>"
        )?;
        Ok(Self {
            path: path.to_owned(),
            idprefix: idprefix.to_owned(),
        })
    }

    fn write_xml(&self, out: &mut String) {
        let _ = writeln!(
            out,
            r#"  <SetDefaults path="{}" idprefix="{}" />"#,
            self.path, self.idprefix
        );
    }
}
pub struct ManifestItemRef<'a> {
    pub full_id: String,
    pub full_path: String,
    pub item: &'a ManifestItem,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ManifestItem {
    pub tag: ManifestItemTag,
    pub id: String, // in raw, this is the xml to write
    pub path: String,
    pub attributes: BTreeMap<String, String>,
}
impl ManifestItem {
    pub fn new(tag: ManifestItemTag, id: String, path: String) -> Self {
        Self {
            tag,
            id,
            path,
            attributes: Default::default(),
        }
    }
    pub fn raw(xml: String) -> Self {
        Self {
            tag: ManifestItemTag::Raw,
            id: xml,
            path: Default::default(),
            attributes: Default::default(),
        }
    }
    fn parse(node: Node) -> cu::Result<Self> {
        let tag = cu::check!(
            ManifestItemTag::parse(node.tag_name().name()),
            "failed to parse resource item tag"
        )?;
        let id = cu::check!(
            node.attribute("id"),
            "missing `id` attribute on resource item"
        )?;
        let path = cu::check!(
            node.attribute("path"),
            "missing `path` attribute on resource item"
        )?;

        // collect every attribute besides `id` and `path`; an empty value is
        // a flag (`None`), a non-empty value is kept as `Some`
        let mut attributes = BTreeMap::new();
        for attr in node.attributes() {
            let name = attr.name();
            if name == "id" || name == "path" {
                continue;
            }
            let value = attr.value();
            attributes.insert(name.to_owned(), value.to_owned());
        }

        Ok(Self {
            tag,
            id: id.to_owned(),
            path: path.to_owned(),
            attributes,
        })
    }

    pub fn is_raw(&self) -> bool {
        self.tag == ManifestItemTag::Raw
    }

    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attributes.get(name).map(|x| x.as_str())
    }

    pub fn attr_bool(&self, name: &str) -> bool {
        self.attributes.contains_key(name)
    }

    pub fn set_attr(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.attributes.insert(name.into(), value.into());
    }

    fn write_xml(&self, out: &mut String) {
        if self.is_raw() {
            let _ = writeln!(out, "{}", self.id);
            return;
        }
        let _ = write!(
            out,
            r#"  <{} id="{}" path="{}""#,
            self.tag.to_str(),
            self.id,
            self.path
        );
        for (key, value) in &self.attributes {
            let _ = write!(out, r#" {key}="{value}""#);
        }
        let _ = writeln!(out, " />");
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum ManifestItemTag {
    Raw,
    Image,
    Font,
    Sound,
}
impl ManifestItemTag {
    fn parse(name: &str) -> cu::Result<Self> {
        match name {
            "Image" => Ok(Self::Image),
            "Font" => Ok(Self::Font),
            "Sound" => Ok(Self::Sound),
            other => cu::bail!("invalid resource item tag <{other}>"),
        }
    }
    pub fn to_str(self) -> &'static str {
        match self {
            ManifestItemTag::Raw => "Raw",
            ManifestItemTag::Image => "Image",
            ManifestItemTag::Font => "Font",
            ManifestItemTag::Sound => "Sound",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // minimal manifest covering the structure: two segments in one group
    // (a leading items-only segment, then a <SetDefaults> segment), a flag
    // attribute, a valued attribute, and all three item tags.
    const SAMPLE: &str = r#"<?xml version="1.0"?>
<ResourceManifest>
  <Resources id="Sample">
    <Image id="BLANK" path="blank" />
    <SetDefaults path="images" idprefix="IMAGE_" />
    <Image id="SEEDS" path="seeds" cols="9" />
    <Image id="TITLESCREEN" path="titlescreen" a8r8g8b8="" />
    <Font id="BRIANNETOD16" path="BrianneTod16.txt" />
    <Sound id="BUTTONCLICK" path="buttonclick" />
  </Resources>
</ResourceManifest>
"#;

    #[test]
    fn test_parse_sample() {
        let manifest = Manifest::try_parse_xml(SAMPLE).expect("should parse sample manifest");

        assert_eq!(manifest.resource_groups.len(), 1);

        // the group has two segments: a leading items-only one (no defaults)
        // and one introduced by <SetDefaults>
        let group = &manifest.resource_groups[0];
        assert_eq!(group.id, "Sample");
        assert_eq!(group.items_with_defaults.len(), 2);
        assert!(group.items_with_defaults[0].set_defaults.is_none());
        let defaults = group.items_with_defaults[1]
            .set_defaults
            .as_ref()
            .expect("second segment should have defaults");
        assert_eq!(defaults.path, "images");
        assert_eq!(defaults.idprefix, "IMAGE_");

        // SEEDS has a valued attribute cols="9"
        let seeds = find_item(&manifest, "SEEDS").expect("SEEDS item should exist");
        assert!(matches!(seeds.tag, ManifestItemTag::Image));
        assert_eq!(seeds.path, "seeds");
        assert_eq!(seeds.attr("cols"), Some("9"));

        // TITLESCREEN has a flag attribute a8r8g8b8=""
        let titlescreen = find_item(&manifest, "TITLESCREEN").expect("TITLESCREEN should exist");
        assert!(titlescreen.attr_bool("a8r8g8b8"));

        // a Sound and a Font should parse with the right tag
        let buttonclick = find_item(&manifest, "BUTTONCLICK").expect("BUTTONCLICK should exist");
        assert!(matches!(buttonclick.tag, ManifestItemTag::Sound));
        let font = find_item(&manifest, "BRIANNETOD16").expect("BRIANNETOD16 should exist");
        assert!(matches!(font.tag, ManifestItemTag::Font));
    }

    #[test]
    fn test_to_xml() {
        let manifest = Manifest::try_parse_xml(SAMPLE).expect("should parse sample manifest");

        let expected = r#"<?xml version="1.0"?>
<ResourceManifest>

<Resources id="Sample">
  <Image id="BLANK" path="blank" />

  <SetDefaults path="images" idprefix="IMAGE_" />
  <Image id="SEEDS" path="seeds" cols="9" />
  <Image id="TITLESCREEN" path="titlescreen" a8r8g8b8="" />
  <Font id="BRIANNETOD16" path="BrianneTod16.txt" />
  <Sound id="BUTTONCLICK" path="buttonclick" />

</Resources>

</ResourceManifest>
"#;
        assert_eq!(manifest.to_xml(), expected);
    }

    #[test]
    fn test_roundtrip() {
        let manifest = Manifest::try_parse_xml(SAMPLE).expect("should parse sample manifest");
        let xml = manifest.to_xml();
        let reparsed = Manifest::try_parse_xml(&xml).expect("emitted xml should re-parse");
        assert_eq!(manifest, reparsed);
    }

    fn find_item<'a>(manifest: &'a Manifest, id: &str) -> Option<&'a ManifestItem> {
        manifest
            .resource_groups
            .iter()
            .flat_map(|g| &g.items_with_defaults)
            .flat_map(|s| &s.items)
            .find(|item| item.id == id)
    }
}
