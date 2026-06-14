// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Pistonight/pvz-bintools contributors

#[cu::cli]
fn main(cli: pvz_reanimc::Cli) -> cu::Result<()> {
    pvz_reanimc::run(cli)
}
