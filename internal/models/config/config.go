package config

import (
	"fmt"

	"github.com/BurntSushi/toml"
)

// Project-global sink configuration.
//
// Internal representation of sink.toml
type Config struct {
	Defaults Defaults

	// --------------------- //
	// NON-TOML FIELDS BELOW //
	// --------------------- //

	ProjectRoot string
	IsProject   bool
}

// Global defaults to apply to every sink operation / file.
type Defaults struct {
	// Whether to add the sinked file to .gitignore
	Gitignore bool

	// Whether to add the release-based file to sink.lock
	Lock bool

	// Default source configuration for release-based sinks
	// TODO: Think this through
	Source struct {
		// The owner of the [Defaults.Source.Repository]
		Owner string

		// The default repository to download assets from
		Repository string
	}
}

// Create a new config with defaults applied
func NewConfigWithDefaults() Config {
	return Config{
		Defaults: Defaults{
			Gitignore: true,
			Lock:      true,
		},
	}
}

// Parse and load the given file into a [Config] struct
func (c *Config) LoadConfig(file string) error {
	md, err := toml.DecodeFile(file, &c)
	if undecoded := md.Undecoded(); len(undecoded) > 0 {
		err = fmt.Errorf("encountered unknown keys: %s", undecoded)
	}
	if err != nil {
		return err
	}

	return nil
}
