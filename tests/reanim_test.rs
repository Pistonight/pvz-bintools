use clap::Parser as _;
use cu::pre::*;
use pvz_bintools::cli::{self, Cli};

// 1051 and 1065 doesn't have the original reanim files

#[test]
fn test_reanim_1073() -> cu::Result<()> {
    test_reanim("main1073")
}

#[test]
fn test_reanim_1096() -> cu::Result<()> {
    test_reanim("main1096")
}

fn test_reanim(name: &str) -> cu::Result<()> {
    let test_output = format!("data/{name}_reanim_test");
    let unpack = format!("data/{name}");

    // the "original_sources" are at <pak>/reanim/*.reanim
    let original_sources = format!("{unpack}/reanim/*.reanim");
    // the "original_compiled" are at <pak>/compiled/reanim/*.reanim.compiled
    let original_compiled = format!("{unpack}/compiled/reanim/*.reanim.compiled");

    let our_compiled = format!("{test_output}/our_compiled");
    let orig_dump = format!("{test_output}/orig_dump");
    let our_dump = format!("{test_output}/our_dump");

    let manifest = format!("{unpack}/properties/resources.xml");

    // compile original using our compiler
    run_cli(&[
        "pvz-bintools",
        "reanimc",
        &original_sources,
        "-o",
        &our_compiled,
        "--manifest",
        &manifest,
        "--pak-dir",
        &unpack,
    ])?;

    // dump original
    run_cli(&[
        "pvz-bintools",
        "reanimc",
        &original_compiled,
        "--dump",
        "-o",
        &orig_dump,
    ])?;
    // dump ours
    run_cli(&[
        "pvz-bintools",
        "reanimc",
        &format!("{our_compiled}/*.reanim.compiled"),
        "--dump",
        "-o",
        &our_dump,
    ])?;

    // diff original and our compiled dump
    let dir = cu::fs::read_dir(&orig_dump)?;
    let mut count = 0;
    let mut has_diff = false;
    for entry in dir {
        let entry = entry?;
        let name = entry.file_name().into_utf8()?;
        let our = cu::path!(&our_dump / &name);
        let original_bytes = cu::fs::read(entry.path())?;
        let our_bytes = cu::fs::read(our)?;
        if original_bytes != our_bytes {
            has_diff = true;
            cu::error!("diff found for '{name}'");
        }
        count += 1;
    }

    cu::ensure!(
        count > 0,
        "no dumped files found to compare in '{orig_dump}'"
    )?;
    cu::ensure!(
        !has_diff,
        "compiled output differs from original for '{name}'"
    )?;

    Ok(())
}

fn run_cli(args: &[&str]) -> cu::Result<()> {
    let mut cli = Cli::try_parse_from(args)?;
    cli.preprocess();
    cli::run(cli)
}
