# Usage

Running `sink --help` will output the following:

```shell
sink --help
```

## CLI structure

```shell
sink

    --help:         Show this message
    --verbose:      Increase verbosity of sink
    --file:         The sink file to use. Defaults to 'sink.toml'.

    config              Interact with the sink TOML
        -a, --all:      DEBUGGING ONLY: Shows the entire config (as a structure)
        -t, --toml:     DEBUGGING ONLY: Shows the entire config as TOML (with includes resolved)
        -l, --list:     List all dependencies
        -u, --update:   Update the value of a config field. This is not intended to be used on dependencies

    install             Install all dependencies
        -s, --sink:     Optional, Install based on sink.lock

    add <dependency>                Add and install a dependency in the form of 'owner/repo:dependency'
        -d, --dest, --destination:  Optional, The local destination to download the file(s) into
        -v, --version:              Optional, The version (git tag) to download
        --no-gitignore:             Optional, Do not add the dependency to the .gitignore file

    remove <dependency>             Remove and uninstall a dependency in the form of 'owner/repo:dependency'
```
