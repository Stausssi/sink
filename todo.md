# Open topics / To Do / Implementation plan

- [x] Init Go project
- [x] CLI framework: Cobra
- [x] Package structure: DDD, ...? -> Only initial proposal, let's get code first and then see if it needs to be refactored.

## Application

- [x] Path traversal
  - [x] find project root based on .git
  - [x] find .sink files in the project
- [x] parse .sink files
- [ ] backend for downloading files
  - [ ] git-based (git archive or sparse checkout?)
  - [ ] release-based (authentication?)
  - [ ] Go packages to reuse? (like go-git, go-github, ...)
- [ ] Sinkfile validation (with <https://pkg.go.dev/github.com/go-playground/validator/v10>)
- [ ] Use [`go-digest`](https://github.com/opencontainers/go-digest) for checksum

## CLI

- [ ] CLI structure
- [ ] subcommands and flags
- [x] decide terminal framework: [`charmbracelet/bubbletea`](https://github.com/charmbracelet/bubbletea)

## Config

- [x] Go TOML parser for default config
- [ ] Use viper for automatic config handling via file & CLI

## Other

- [x] Recommended VS Code Extensions
- [ ] `settings.json`
- [ ] Shared action(s)
