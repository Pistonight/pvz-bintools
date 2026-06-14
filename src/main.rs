// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Pistonight/pvz-bintools contributors

use pvz_bintools::cli;

#[cu::cli(preprocess=cli::Cli::preprocess)]
fn main(args: cli::Cli) -> cu::Result<()> {
    cli::run(args)
}
