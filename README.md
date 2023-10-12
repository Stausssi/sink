# sink

`sink` is an universal and extensible package manager.

It provides limitless extensibility through a modular plugin system, supporting your programming language of choice.

## Documentation

See [`docs`](docs/index.md) for detailed documentation.

## Installation

- (linux)`brew`
- `cargo`?

## Usage

```shell
sink install
```

## Configuration

The configuration of `sink` is done on a project basis via a `sink.toml` file.
This file contains all dependencies from various sources and general configurations.
It can also be split up into multiple files, e.g. to separate programming languages.

## Plugins

Plugins are essential for `sink`.
They bridge `sink.toml` with a dependency backend such as `pip`, `cargo`, and so on.

Thus, it's also possible to convert `sink.toml` entries into the native form, e.g. `requirements.txt`, `Cargo.toml`, etc..
This way, the default package manager can still be used as a fallback in environments where `sink` is not present.
