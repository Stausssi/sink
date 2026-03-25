package config

import (
	"github.com/BurntSushi/toml"
)

// Project-global sink configuration.
//
// Internal representation of sink.toml
type Config struct {
	Defaults Defaults
}

// Global defaults to apply to every sink operation / file.
type Defaults struct {
	// Whether to add the sinked file to .gitignore
	Gitignore bool

	// Whether to add the release-based file to sink.lock
	Lock bool

	// Default source configuration for release-based sinks
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
		Defaults{
			Gitignore: true,
			Lock:      true,
		},
	}
}

// Parse and load the given file into a [Config] struct
func (c *Config) LoadConfig(file string) error {
	_, err := toml.DecodeFile(file, &c)
	if err != nil {
		return err
	}
	return nil
}
