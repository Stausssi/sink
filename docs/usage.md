# Usage

Running `sink --help` will output the following:

```shell
sink --help
```

## CLI structure

```shell
sink

    --verbose:      Increase verbosity of sink

    config              Interact with the sink TOML
        -a, --all:      Shows the entire config (as a structure)
        -t, --toml:     Shows the entire config as TOML (with includes resolved)
        -l, --list:     List a type of entry (choice of 'groups', 'langs', 'dependencies')
        -f, --field:    Select a singular field by identifier
        -u, --update:   Update the value of a config field. This is not intended to be used on dependencies

    install             Install dependencies
        -a, --all:      All languages and dependencies
        -l, --lang:     Only specific language/type. Can be combined with group.
        -g, --group:    Only dependency group (e.g. dev). Can be combined with lang.
        -s, --sink:     Install based on sink.lock

    add                                 Add and install a dependency
        [lang]/[dependency]:            Shorthand for 'sink [lang] add [dependency]'. Args will be passed to the corresponding command.
        [lang]/[dependency]@[version]:  Shorthand for 'sink [lang] add [dependency] --version [version]'. Other args will be forwarded.

    remove                      Remove and uninstall a dependency
        [lang]/[dependency]:    Shorthand for 'sink [lang] remove [dependency]'. Args will be passed to the corresponding command.

    python
        add                 Add and install a dependency
            [dependency]    The name of the dependency
            -v, --version:  The version to install
            -e, --extras:   The extras to install of the dependency
            -u, --url:      The URL of the package. Can be used for git or other installs.
            -g, --group:    The group to install the dependency in
        remove              Remove and uninstall a dependency
            [dependency]    The name of the dependency

    rust
        add                 Add and install a dependency
            [dependency]    The name of the dependency
            -v, --version:  The version to install
            -f, --features: The features to install
            -g, --group:    The group to install the dependency in
        remove              Remove and uninstall a dependency
            [dependency]    The name of the dependency

    github                              Manage GitHub dependencies
    gh                                  Shorthand for 'github'
        add                             Add and install a dependency
            [dependency]                The name of the dependency in the form of 'owner/repo/dependency'
            -d, --dest, --destination:  The local destination to download the file(s) into
            -v, --version:              The version (git tag) to download
            -g, --group:                The group to install the dependency in
        remove                          Remove and uninstall a dependency
            [dependency]    The name of the dependency in the form of 'owner/repo/dependency'
```
