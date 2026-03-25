package cmd

import (
	"errors"
	"io/fs"
	"os"
	"path/filepath"
	config "sink/internal"
	"sink/internal/application/pathfinder"

	"github.com/spf13/cobra"
)

var ProjectRoot string
var FileConfig = config.NewConfigWithDefaults()
var Verbose bool

var rootCmd = &cobra.Command{
	Use:   "sink",
	Short: "A lightweight package manager for sharing files and assets across repositories",
	Long: `A lightweight package manager for sharing files and assets across repositories.

It helps to re-use one file from a given repository in another one with as little overhead and setup as possible.`,
	PersistentPreRunE: func(cmd *cobra.Command, args []string) error {
		determinedRoot, err := pathfinder.ProjectRoot()
		if err != nil {
			return err
		}
		ProjectRoot = determinedRoot

		err = FileConfig.LoadConfig(filepath.Join(determinedRoot, "sink.toml"))
		if errors.Is(err, fs.ErrNotExist) {
			return nil
		}
		return err
	},
}

// Execute adds all child commands to the main command and sets flags appropriately.
// This is called by main.main(). It only needs to happen once to the rootCmd.
func Execute() {
	err := rootCmd.Execute()
	if err != nil {
		os.Exit(1)
	}
}

func init() {
	// Here you will define your flags and configuration settings.
	// Cobra supports persistent flags, which, if defined here,
	// will be global for your application.

	rootCmd.PersistentFlags().BoolVarP(&Verbose, "verbose", "v", false, "verbose output")
}
