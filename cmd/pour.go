package cmd

import (
	"fmt"
	"os"
	"sink/internal/application/pour"

	tea "charm.land/bubbletea/v2"
	"github.com/spf13/cobra"
)

var pourCmd = &cobra.Command{
	Use:     "pour",
	Aliases: []string{"install"},
	Short:   "Pour (install) files",
	Long: `Pour (install) files.

Downloads all .sink files from their sources and places them in the appropriate locations.`,
	Run: func(cmd *cobra.Command, args []string) {
		m, err := tea.NewProgram(pour.NewPourModel(FileConfig)).Run()
		model := m.(pour.PourModel)
		if err != nil {
			fmt.Printf("Uh oh, that's not supposed to happen: %v\n", err)
			os.Exit(1)
		}
		os.Exit(model.ExitCode)
	},
}

func init() {
	rootCmd.AddCommand(pourCmd)
}
