# run-all

[![Build Status](https://travis-ci.org/headexplodes/run-all.svg?branch=master)](https://travis-ci.org/headexplodes/run-all)

Run multiple commands and interleave their output into a single terminal. Designed for running multiple services locally during development, without any special tooling or multiple terminal tabs.

## Usage

Pass one or more commands to run, with an optional alias before each command.

```bash
run-all [{-a|--alias} <alias>] <command> ...
```

## Installing

Requires Rust and Cargo, see https://www.rust-lang.org/tools/install.

To download, compile and install:

```bash
cargo install --git https://github.com/headexplodes/run-all.git
```

Assuming you have `~/.cargo/bin` in your `PATH`, to run command `run-all`.

```bash
run-all --alias example ./example.sh
```
