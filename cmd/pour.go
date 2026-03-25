package cmd

import (
	"fmt"
	"sink/internal/application/pathfinder"
	"sink/internal/application/sinkfile"
	"strings"

	"github.com/spf13/cobra"
)

var pourCmd = &cobra.Command{
	Use:     "pour",
	Aliases: []string{"install"},
	Short:   "Pour (install) files",
	Long: `Pour (install) files.

Downloads all .sink files from their sources and places them in the appropriate locations.`,
	Run: func(cmd *cobra.Command, args []string) {
		// fmt.Printf("Sink config: %+v\n", FileConfig)
		sinkFilePaths := pathfinder.FindSinkFiles(ProjectRoot, pathfinder.IsSinkProject(ProjectRoot))

		sinkFiles := make([]sinkfile.Sinkfile, 0, len(sinkFilePaths))
		for _, path := range sinkFilePaths {
			parsedFile, err := sinkfile.ParseSinkfile(FileConfig, path)
			if err != nil {
				fmt.Printf("Failed to parse %s: %s", path, err)
				continue
			}
			sinkFiles = append(sinkFiles, parsedFile)
		}

		if len(sinkFilePaths) <= 0 {
			fmt.Println("No files to sink.")
			return
		}
		fmt.Println("🚰 Pouring files...")
		for _, sinkFile := range sinkFiles {
			targetPath := strings.TrimSuffix(sinkFile.GetGeneralOptions().SinkfilePath, ".sink")
			fmt.Printf("💧 Pouring %s ...\n", targetPath)
			sinkFile.Get(targetPath)

		}
		fmt.Println("🔧 All files poured.")
	},
}

func init() {
	rootCmd.AddCommand(pourCmd)
}
