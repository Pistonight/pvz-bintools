use clap::Parser as _;
use cu::pre::*;

use pvz_bintools::cli::{self, Cli};
use pvz_bintools::tool::resc::Manifest;

#[test]
fn test_resc_1051() -> cu::Result<()> {
    test_manifest("1051")
}

#[test]
fn test_resc_1065() -> cu::Result<()> {
    test_manifest("1065")
}

#[test]
fn test_resc_1073() -> cu::Result<()> {
    test_manifest("1073")
}

#[test]
fn test_resc_1096() -> cu::Result<()> {
    test_manifest("1096")
}

fn test_manifest(name: &str) -> cu::Result<()> {
    let config = format!("data/resources{name}.yaml");
    let original_xml = format!("data/main{name}/properties/resources.xml");
    let test_dir = format!("data/main{name}_test/resc");
    let output_xml = format!("{test_dir}/resources.xml");

    run_cli(&["pvz-bintools", "resc", &config])?;

    let mut our_manifest = Manifest::try_parse_xml(&cu::fs::read_string(&output_xml)?)?;
    our_manifest.sort();
    let mut orig_manifest = Manifest::try_parse_xml(&cu::fs::read_string(&original_xml)?)?;
    orig_manifest.sort();
    let our_json = json::stringify_pretty(&our_manifest)?;
    let orig_json = json::stringify_pretty(&orig_manifest)?;
    cu::fs::write(format!("{test_dir}/our_resources.json"), our_json)?;
    cu::fs::write(format!("{test_dir}/orig_resources.json"), orig_json)?;
    if our_manifest != orig_manifest {
        cu::bail!("differences found in {name}");
    }

    Ok(())
}

fn run_cli(args: &[&str]) -> cu::Result<()> {
    let mut cli = Cli::try_parse_from(args)?;
    cli.preprocess();
    cli::run(cli)
}
