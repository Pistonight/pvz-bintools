# pvz-bintools

Command line binary tools for PVZ. Currently only supported on Windows (due to the timestamps stored in pak files).

This is useful to automate modding workflows.

## Installation
The GitHub release has a windows executable built on my machine.

You can also install and build from source with a Rust toolchain (but note only Windows is supported right now)
```
cargo install --git https://github.com/Pistonight/pvz-bintools --locked
```
(Remove `--locked` if it doesn't compile)

## Usage
Run in terminal: (does not have a GUI)
```
pvz-bintools [COMMAND] [ARGS...]
```

Currently it has the following tools:

- `pakc` - Pack and unpack `.pak` files. This tool can unpack original game assets and pack them back to the exact same bytes
- `reanimc` - Compile `.reanim` to `.reanim.compiled`, also dumping to JSON for inspection

Run `pvz-bintools COMMAND --help` for detailed usage (for example `pvz-bintools pakc --help`)

May be added in the future:
- strings tool
- resource tool
- particle compiler
