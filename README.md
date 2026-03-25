# sink

`sink` is a lightweight package manager to keep shared files and assets across repositories in *✨ sink ✨*.

It aims to solve a problem I experienced one too many times: Re-use one file from a given repository in another one with as little overhead as possible.
See [Why `sink`](#why-sink) for more details.

## Quick start

Getting started with `sink` is quite easy.
Simply decide which file you want to *sink* and create a file in the **target repository**:

```terminal
> cat demo.md.sink
source=https://github.com/Stausssi/sink:README.md
ref=main

> sink pour
🚰 Pouring files...
💧 Pouring demo.md ... Done
🔧 All files poured. Tap is now closed!

> cat demo.md
# sink
...
```

That's it!
And you will also notice that `demo.md` was also automatically added to the `.gitignore`.

> [!TIP]
> In the beginning, `sink install` is probably easier to remember than `sink pour`.

See `sink --help` for more options and features.

## Installation

- [ ] Go install
- [ ] Homebrew
- [ ] Download binary from releases

## `.sink` file spec

I tried to keep `.sink` files as simple as possible, but also flexible enough to support a wide range of use cases.
They follow INI-file spec

There's actually two ways to specify how and where to download a file from: [Git based](#git-based) and [Release based](#release-based).

> [!TIP]
> Comments are supported in `.sink` files using `#`.

### Git based

As shown in the [quick start](#quick-start), git-based downloads use `source` and `ref` to specify the source of the file and the git ref to download from.
This is basically like a `git checkout` of a specific file from a given repository and ref.

```ini
# Where to get the file from in the form of `<repository>:<file path>`
source=https://github.com/Stausssi/sink:README.md

# Git ref to download from.
# Can be a branch, tag or commit hash.
ref=main
```

> [!TIP]
> `github.com` is the default host and can be omitted.
> For the example above `Stausssi/sink:README.md` would suffice.

### Release based

Release-based downloads are similar to git-based ones, but instead of specifying a git ref, you specify a GitHub release version.
It can be useful, especially coupled with [immutable releases](https://docs.github.com/en/code-security/concepts/supply-chain-security/immutable-releases), to ensure that the file you are downloading doesn't change unexpectedly.

To download from a release instead of a git reference, simply specify `version` instead of `ref`:

```ini
# Where to get the file from in the form of `<GitHub repository>:<asset name>`
source=Stausssi/sink:README.md

# GitHub release version to download from.
# This can be a version tag or one of 'latest' or 'latest-unstable' (incl. pre-releases).
version=latest

# Optional SHA 256 checksum of the file to verify integrity after download.
checksum=abc123...
```

### Common options

Besides the source and ref/version, there are some common options that can be used for both git-based and release-based downloads:

```ini
# ... source and ref/version spec goes here ...

# Toggle automatic .gitignore update.
# By default, sink will add the downloaded file to .gitignore to avoid accidentally committing it.
gitignore=true

# Whether to add this file to the lock file.
# Defaults to true for release-based downloads and false for git-based ones, but can be overridden for both.
lock=true

# Optionally symlink the file to other locations relative to the current file.
# Especially useful for AGENTS.md compatibility.
# Can be specified multiple times.
symlink=CLAUDE.md
symlink=.github/copilot-instructions.md

# Optional file permissions to set on the downloaded file.
# Specified in octal format and applied after download.
# Defaults to 644.
permissions=640
```

## Project defaults

The [`.sink` file spec](#sink-file-spec) can become quite verbose when you have to specify multiple files from the same owner/repository over and over again or simply want to control project-wide default behaviour.
That's why `sink` also supports a top-level `sink.toml` configuration file:

```toml
[defaults]
gitignore = false
lock      = false

[defaults.source]
owner = "Stausssi"
repository = "sink"
```

## Lock file

If any of the *sinked* files have locking enabled, a `sink.lock` file will be created in the top-level directory to keep track of the exact version of the file that was downloaded:

TODO: Specify format and content of the lock file.

## Why `sink`?

It's quite simple; whenever I had to re-use files across repositories and most importantly keep them synchronised, both submodules and subtrees felt too clunky, complicated to setup and also didn't really fit my use case that well.
I just wanted a lightweight solution that is easy to use and doesn't require a lot of overhead to set up and maintain.
Real package managers felt like overkill for just sharing a few files across repositories sporadically, especially when dealing with non-code files, configuring registries, ...

And I also just like to build things and wanted to learn Go, so here we are!
