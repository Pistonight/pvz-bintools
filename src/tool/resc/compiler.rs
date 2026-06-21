// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Pistonight/pvz-bintools contributors

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use cu::{fs::WalkEntry, pre::*};

use crate::tool::resc::{
    Config, Config_font_item, Config_image_item, Config_item_path, Config_resource_content_common,
    Config_resource_content_font, Config_resource_content_image, Config_resource_content_sound,
    Config_resource_group, Config_resource_group_content, Config_sound_item, Manifest,
    ManifestItem, ManifestItemTag, ManifestItemsWithDefault, ManifestResourceGroup,
    ManifestSetDefaults,
};
use crate::util::Pattern;

static IMAGE_EXTENSIONS: [&str; 3] = [".png", ".jpg", ".gif"];
static FONT_EXTENSIONS: [&str; 2] = [".txt", ".ttf"];

pub fn compile(config: &Config) -> cu::Result<Manifest> {
    let mut entries = cu::check!(read_entries(config), "failed to read input directory")?;
    cu::trace!("all entries: {entries:#?}");
    let all_paths = entries.clone();
    let mut id_to_paths = Default::default();

    let mut groups = Vec::with_capacity(config.resource_groups.len());
    for group in &config.resource_groups {
        let compiled_group = compile_group(&all_paths, &mut entries, &mut id_to_paths, group)?;
        groups.push(compiled_group);
    }

    let manifest = Manifest {
        resource_groups: groups,
    };
    Ok(manifest)
}

fn read_entries(config: &Config) -> cu::Result<BTreeSet<String>> {
    let mut allpaths = BTreeSet::new();
    for input_directory in &config.paths.input_directories {
        cu::info!("processing resource directory: {input_directory}");
        let mut current_paths = BTreeSet::new();
        let mut walk = cu::fs::walk_with(input_directory, |entry: &WalkEntry| {
            let path = entry.rel_path();
            let Ok(id) = normalize_entry(&path) else {
                return false;
            };
            for p in &config.paths.excludes {
                if p.matches(&id) {
                    cu::debug!("excluded directory {id}");
                    return false;
                }
            }
            true
        })?;
        'outer: while let Some(entry) = walk.next() {
            let entry = entry?;
            let path = entry.rel_path();
            let norm_path = cu::check!(
                normalize_entry(&path),
                "failed to normalize entry path: '{}'",
                path.display()
            )?;

            for p in &config.paths.excludes {
                if p.matches(&norm_path) {
                    cu::debug!("excluded {norm_path}");
                    continue 'outer;
                }
            }
            current_paths.insert(norm_path);
        }

        for p in allpaths.intersection(&current_paths) {
            cu::warn!("resource appears in multiple input directories: {p}");
        }
        allpaths.extend(current_paths);
    }
    Ok(allpaths)
}

fn normalize_entry(rel: &Path) -> cu::Result<String> {
    if !rel.is_relative() {
        cu::bail!("path must be within the input directory");
    }
    let rel = cu::check!(rel.as_utf8(), "path must be UTF8")?;
    let rel = rel.trim_start_matches(['.', '/', '\\']);

    let norm = rel.replace('\\', "/");

    Ok(norm)
}

fn compile_group(
    all_paths: &BTreeSet<String>,
    paths: &mut BTreeSet<String>,
    id_to_paths: &mut BTreeMap<String, String>,
    group: &Config_resource_group,
) -> cu::Result<ManifestResourceGroup> {
    let mut items = Vec::with_capacity(group.contents.len());
    for content in &group.contents {
        let compiled = cu::check!(
            compile_groupped_content(all_paths, paths, id_to_paths, content),
            "failed to compile group {}",
            group.name
        )?;
        items.push(compiled);
    }
    Ok(ManifestResourceGroup {
        id: group.name.clone(),
        items_with_defaults: items,
    })
}

fn compile_groupped_content(
    all_paths: &BTreeSet<String>,
    paths: &mut BTreeSet<String>,
    id_to_paths: &mut BTreeMap<String, String>,
    content: &Config_resource_group_content,
) -> cu::Result<ManifestItemsWithDefault> {
    match content {
        Config_resource_group_content::Image(content_group) => {
            let mut items = vec![];
            for item in &content_group.items {
                let current_items =
                    compile_image_item(all_paths, paths, id_to_paths, item, content_group)?;
                items.extend(current_items);
            }
            let set_defaults =
                content_group
                    .common
                    .defaults
                    .as_ref()
                    .map(|x| ManifestSetDefaults {
                        path: x.path.clone(),
                        idprefix: x.id_prefix.clone(),
                    });
            Ok(ManifestItemsWithDefault {
                set_defaults,
                items,
            })
        }
        Config_resource_group_content::Font(content_group) => {
            let mut items = vec![];
            for item in &content_group.items {
                let current_items =
                    compile_font_item(all_paths, paths, id_to_paths, item, content_group)?;
                items.extend(current_items);
            }
            let set_defaults =
                content_group
                    .common
                    .defaults
                    .as_ref()
                    .map(|x| ManifestSetDefaults {
                        path: x.path.clone(),
                        idprefix: x.id_prefix.clone(),
                    });
            Ok(ManifestItemsWithDefault {
                set_defaults,
                items,
            })
        }
        Config_resource_group_content::Sound(content_group) => {
            let mut items = vec![];
            for item in &content_group.items {
                let current_items =
                    compile_sound_item(all_paths, paths, id_to_paths, item, content_group)?;
                items.extend(current_items);
            }
            let set_defaults =
                content_group
                    .common
                    .defaults
                    .as_ref()
                    .map(|x| ManifestSetDefaults {
                        path: x.path.clone(),
                        idprefix: x.id_prefix.clone(),
                    });
            Ok(ManifestItemsWithDefault {
                set_defaults,
                items,
            })
        }
    }
}

fn compile_image_item(
    all_paths: &BTreeSet<String>,
    paths: &mut BTreeSet<String>,
    id_to_paths: &mut BTreeMap<String, String>,
    item: &Config_image_item,
    content_group: &Config_resource_content_image,
) -> cu::Result<Vec<ManifestItem>> {
    let mut out = compile_item_by_path(
        all_paths,
        paths,
        id_to_paths,
        &item.path,
        &content_group.common,
        ManifestItemTag::Image,
        &IMAGE_EXTENSIONS,
    )?;
    if matches!(&item.path, Config_item_path::Raw { .. }) {
        return Ok(out);
    }
    let prefix = content_group
        .common
        .defaults
        .as_ref()
        .map(|x| x.path.as_str());

    let attrs = item.attrs.apply(&content_group.attrs);
    for out_item in &mut out {
        if attrs.nopal {
            out_item.set_attr("nopal", "");
        }
        if attrs.ddsurface {
            out_item.set_attr("ddsurface", "");
        }
        if attrs.a4r4g4b4 {
            out_item.set_attr("a4r4g4b4", "");
        }
        if attrs.a8r8g8b8 {
            out_item.set_attr("a8r8g8b8", "");
        }
        if attrs.minsubdivide {
            out_item.set_attr("minsubdivide", "");
        }
        if attrs.rows != 0 && attrs.rows != 1 {
            out_item.set_attr("rows", attrs.rows.to_string());
        }
        if attrs.cols != 0 && attrs.cols != 1 {
            out_item.set_attr("cols", attrs.cols.to_string());
        }
        if let Some(alphaimage) = &item.attrs.alphaimage {
            if matches!(&item.path, Config_item_path::Exact { external: true, .. }) {
                out_item.set_attr("alphaimage", alphaimage);
            } else {
                let alphaimage = cu::check!(
                    find_image(prefix, paths, &out_item.path, alphaimage),
                    "failed to find alphaimage for ID: {} (path={})",
                    out_item.id,
                    out_item.path
                )?;
                if let Some(alphaimage) = alphaimage {
                    out_item.set_attr("alphaimage", alphaimage);
                }
            }
        }
        if let Some(alphagrid) = &item.attrs.alphagrid {
            if matches!(&item.path, Config_item_path::Exact { external: true, .. }) {
                out_item.set_attr("alphagrid", alphagrid);
            } else {
                let alphagrid = cu::check!(
                    find_image(prefix, paths, &out_item.path, alphagrid),
                    "failed to find alphagrid for ID: {} (path={})",
                    out_item.id,
                    out_item.path
                )?;
                if let Some(alphagrid) = alphagrid {
                    out_item.set_attr("alphagrid", alphagrid);
                }
            }
        }
    }

    Ok(out)
}

fn find_image(
    prefix: Option<&str>,
    paths: &BTreeSet<String>,
    item_name: &str,
    target_path: &str,
) -> cu::Result<Option<String>> {
    let target_path = target_path.replace("[name]", item_name);

    let full_path = match prefix {
        Some(prefix) => format!("{prefix}/{target_path}"),
        None => target_path.to_string(),
    };
    if paths.contains(&full_path) {
        for ext in IMAGE_EXTENSIONS {
            if target_path.ends_with(ext) {
                return Ok(Some(remove_ext(&target_path).to_string()));
            }
        }
        cu::bail!("image is not a supported extension: '{target_path}'");
    } else {
        for ext in IMAGE_EXTENSIONS {
            let mut p = full_path.clone();
            p.push_str(ext);
            if paths.contains(&p) {
                return Ok(Some(target_path.clone()));
            }
        }
        cu::bail!("alpha image not found: '{target_path}'");
    }
}

fn compile_font_item(
    all_paths: &BTreeSet<String>,
    paths: &mut BTreeSet<String>,
    id_to_paths: &mut BTreeMap<String, String>,
    item: &Config_font_item,
    content_group: &Config_resource_content_font,
) -> cu::Result<Vec<ManifestItem>> {
    let mut out = compile_item_by_path(
        all_paths,
        paths,
        id_to_paths,
        &item.path,
        &content_group.common,
        ManifestItemTag::Font,
        &FONT_EXTENSIONS,
    )?;
    if matches!(&item.path, Config_item_path::Raw { .. }) {
        return Ok(out);
    }
    if item.sys {
        if let Some(attrs) = &item.attrs {
            for out_item in &mut out {
                if attrs.bold {
                    out_item.set_attr("bold", "");
                }
                if attrs.italic {
                    out_item.set_attr("italic", "");
                }
                if attrs.shadow {
                    out_item.set_attr("shadow", "");
                }
                if attrs.underline {
                    out_item.set_attr("underline", "");
                }
            }
        }
        // the sys fonts need to have !sys:
        for out_item in &mut out {
            out_item.path = format!("!sys:{}", out_item.path);
        }
    } else if !matches!(&item.path, Config_item_path::Exact { external: true, .. }) {
        // the image fonts need to point to the descriptor
        for out_item in &mut out {
            out_item.path.push_str(".txt");
        }
    }
    Ok(out)
}

fn compile_sound_item(
    all_paths: &BTreeSet<String>,
    paths: &mut BTreeSet<String>,
    id_to_paths: &mut BTreeMap<String, String>,
    item: &Config_sound_item,
    content_group: &Config_resource_content_sound,
) -> cu::Result<Vec<ManifestItem>> {
    // sound has no attributes
    compile_item_by_path(
        all_paths,
        paths,
        id_to_paths,
        &item.path,
        &content_group.common,
        ManifestItemTag::Sound,
        &[],
    )
}

fn compile_item_by_path(
    all_paths: &BTreeSet<String>,
    paths: &mut BTreeSet<String>,
    id_to_paths: &mut BTreeMap<String, String>,
    item_path: &Config_item_path,
    content_group_common: &Config_resource_content_common,
    tag: ManifestItemTag,
    filter_extensions: &[&str],
) -> cu::Result<Vec<ManifestItem>> {
    let mut out = vec![];
    let id_prefix = content_group_common
        .defaults
        .as_ref()
        .map(|x| x.id_prefix.as_str())
        .unwrap_or_default();
    let path_prefix = content_group_common
        .defaults
        .as_ref()
        .map(|x| x.path.as_str());
    match item_path {
        Config_item_path::Raw { raw } => {
            return Ok(vec![ManifestItem::raw(raw.to_owned())]);
        }
        Config_item_path::Exact { external, id, path } => {
            if *external {
                let id = cu::check!(
                    id.as_ref(),
                    "id must be specified when external (path={path})"
                )?;
                let full_id = format!("{id_prefix}{id}");
                if let Some(existing_path) =
                    id_to_paths.insert(full_id.clone(), format!("external:{path}"))
                {
                    cu::bail!(
                        "id conflict: {full_id} previously has path '{existing_path}', adding 'external:{path}'"
                    );
                }
                let item = ManifestItem::new(tag, id.to_string(), path.to_string());
                out.push(item);
            } else {
                let full_path = match path_prefix {
                    Some(prefix) => format!("{prefix}/{path}"),
                    None => path.to_string(),
                };
                if !paths.remove(&full_path) {
                    if all_paths.contains(&full_path) {
                        cu::warn!(
                            "{} item with path '{}' used by multiple IDs",
                            tag.to_str(),
                            full_path
                        );
                    } else {
                        cu::bail!(
                            "cannot find {} item with path '{}'",
                            tag.to_str(),
                            full_path
                        );
                    }
                }
                let path_attr = compile_path_attribute(path_prefix, path);
                let id = id
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| path_to_id(path_attr));
                let full_id = format!("{id_prefix}{id}");
                if let Some(existing_path) = id_to_paths.insert(full_id.clone(), full_path.clone())
                {
                    cu::bail!(
                        "id conflict: {full_id} previously has path '{existing_path}', adding '{full_path}'"
                    );
                }
                let item = ManifestItem::new(tag, id, path_attr.to_string());
                out.push(item);
            }
        }
        Config_item_path::Pattern { pattern } => {
            let matched_paths = match_paths(paths, path_prefix, pattern, filter_extensions);
            if matched_paths.is_empty() {
                let mut retry_no_filter_paths = paths.clone();
                let matched_paths2 =
                    match_paths(&mut retry_no_filter_paths, path_prefix, pattern, &[]);
                if !matched_paths2.is_empty() {
                    cu::warn!(
                        "pattern '{pattern}' matched other files with unsupported extensions for {}",
                        tag.to_str()
                    );
                }
                let mut all_paths = all_paths.clone();
                let matched_paths3 =
                    match_paths(&mut all_paths, path_prefix, pattern, filter_extensions);
                if !matched_paths3.is_empty() {
                    cu::warn!(
                        "pattern '{pattern}' matched but all matched files have been used. Each file can only be matched with one pattern."
                    );
                }
                cu::bail!("pattern '{}' matched no {} items", pattern, tag.to_str());
            }
            for path in matched_paths {
                let path_attr = compile_path_attribute(path_prefix, &path);
                let id = path_to_id(path_attr);
                let full_id = format!("{id_prefix}{id}");
                if let Some(existing_path) = id_to_paths.insert(full_id.clone(), path.clone()) {
                    cu::bail!(
                        "id conflict: {full_id} previously has path '{existing_path}', adding '{path}' (matched from pattern)"
                    );
                }
                let item = ManifestItem::new(tag, id, path_attr.to_string());
                out.push(item);
            }
        }
    }
    Ok(out)
}

fn match_paths(
    paths: &mut BTreeSet<String>,
    prefix: Option<&str>,
    pattern: &Pattern,
    filter_extensions: &[&str],
) -> Vec<String> {
    let mut out = vec![];
    let mut current = BTreeSet::new();
    std::mem::swap(paths, &mut current);
    for path in current {
        if !filter_extensions.is_empty() {
            if filter_extensions.iter().all(|ext| !path.ends_with(ext)) {
                paths.insert(path);
                continue;
            }
        }
        let match_part = match prefix {
            Some(prefix) => {
                match path.strip_prefix(prefix) {
                    Some(remain) => remain,
                    None => {
                        // path does not start with prefix
                        paths.insert(path);
                        continue;
                    }
                }
            }
            None => &path,
        };
        // the prefix must match a sub directory
        let Some(match_part) = match_part.strip_prefix('/') else {
            continue;
        };
        let match_part = match_part.trim_start_matches('/');
        if pattern.matches(match_part) {
            out.push(path);
        } else {
            paths.insert(path);
        }
    }

    out
}

fn compile_path_attribute<'a>(prefix: Option<&str>, path: &'a str) -> &'a str {
    let path = match prefix {
        None => path,
        Some(prefix) => path
            .strip_prefix(prefix)
            .unwrap_or(path)
            .trim_start_matches('/'),
    };
    // remove extension
    remove_ext(path)
}

fn remove_ext(path: &str) -> &str {
    match path.rfind('.') {
        None => path,
        Some(x) => &path[..x],
    }
}

fn path_to_id(path: &str) -> String {
    let mut x = String::with_capacity(path.len() * 2);
    for c in path.chars() {
        if c.is_alphanumeric() {
            x.push(c);
        } else if c == '/' {
            x.push_str("__");
        } else {
            x.push('_');
        }
    }
    x.make_ascii_uppercase();
    x
}
