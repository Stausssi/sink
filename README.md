# sink

`sink` is a package manager for retrieving assets from other GitHub releases.

It allows for decoupled projects solely based on GitHub.

## Documentation

See [`docs`](docs/index.md) for detailed documentation.

## Installation

TBD.

Presumable prebuilt binaries and Homebrew.

## Usage

```shell
sink install
```

## Configuration

The configuration of `sink` is done on a project basis via a `sink.toml` file.
This file contains all dependencies from various sources and general configurations.
It can also be split up into multiple files, e.g. to separate programming languages.
