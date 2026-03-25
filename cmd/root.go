package cmd

import (
	"fmt"
	"os"
	"sink/internal/application/project"
	"sink/internal/models/config"

	tea "charm.land/bubbletea/v2"
	"github.com/spf13/cobra"
)

var FileConfig config.Config
var Verbose bool

var rootCmd = &cobra.Command{
	Use:   "sink",
	Short: "A lightweight package manager for sharing files and assets across repositories",
	Long: `A lightweight package manager for sharing files and assets across repositories.

It helps to re-use one file from a given repository in another one with as little overhead and setup as possible.`,
	PersistentPreRunE: func(cmd *cobra.Command, args []string) error {
		m, err := tea.NewProgram(project.NewProjectRootModel()).Run()
		model := m.(project.ProjectRootModel)
		if err != nil {
			fmt.Printf("Uh oh, that's not supposed to happen: %v\n", err)
			os.Exit(1)
		}
		if model.Fatal != nil {
			fmt.Printf("❌ Error: %s\n", model.Fatal)
			os.Exit(1)
		}
		FileConfig = model.Config
		return nil
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
