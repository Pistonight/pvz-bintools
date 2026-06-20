// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Pistonight/pvz-bintools contributors

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::ffi::{OsStr, OsString};
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use cu::pre::*;

use crate::tool::resc::{Config, Manifest, compiler};

#[derive(Debug, clap::Parser, AsRef)]
pub struct Cli {
    /// Input file. Note that the paths in the YAML config is relative to the containing directory
    /// of the input
    pub input: String,

    /// Run in dump mode instead of compile mode. Dump a resources.xml file to JSON for inspection
    #[clap(short, long)]
    pub dump_xml: bool,

    #[clap(flatten)]
    #[as_ref]
    flags: cu::cli::Flags,
}

pub fn run(cli: Cli) -> cu::Result<()> {
    let input = cu::fs::read_string(&cli.input)?;
    let containing_dir = PathBuf::from(cli.input).parent_abs()?;

    if cli.dump_xml {
        let manifest = cu::check!(
            Manifest::try_parse_xml(&input),
            "failed to parse input manifest"
        )?;
        let manifest_json = cu::check!(
            json::stringify_pretty(&manifest),
            "failed to dump manifest to json"
        )?;
        println!("{manifest_json}");
        cu::lv::disable_print_time();
        return Ok(());
    }

    let mut config = cu::check!(yaml::parse::<Config>(&input), "failed to parse config")?;
    // fix config paths
    config.paths.input_directory = containing_dir
        .join(&config.paths.input_directory)
        .into_utf8()?;
    config.paths.output_xml = containing_dir.join(&config.paths.output_xml).into_utf8()?;
    config.paths.output_cpp = containing_dir.join(&config.paths.output_cpp).into_utf8()?;
    if let Some(output_h) = config.paths.output_h.as_mut() {
        *output_h = containing_dir.join(&*output_h).into_utf8()?;
    }

    let manifest = cu::check!(compiler::compile(&config), "failed to compile config")?;
    let xml = manifest.to_xml();

    cu::check!(
        cu::fs::write(config.paths.output_xml, xml),
        "failed to write resources xml manifest"
    )?;
    cu::info!("written resources xml");

    Ok(())
}
