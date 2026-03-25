package pathfinder

import (
	"errors"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
)

// Checks whether the given path is the root of a sink project.
//
// Determined by checking for a .git directory
func IsSinkProject(path string) bool {
	gitDir, err := os.Stat(filepath.Join(path, ".git"))
	return err == nil && gitDir.IsDir()
}

// Traverses upwards through the filesystem starting at currentPath to find the project root directory (see [IsSinkProject]).
//
// Stops at the user home folder and ultimately at the fs root with both cases returning an error.
// Also returns an error if any of the file operations fail, e.g. due to permission issues or non-existent files.
func findProjectRoot(currentPath string) (string, error) {
	// Path sanitation first to remove any links or invalid paths
	currentPath, err := filepath.Abs(currentPath)
	if err != nil {
		return "", err
	}
	if currentFile, err := os.Stat(currentPath); err != nil {
		return "", err
	} else if !currentFile.IsDir() {
		currentPath = filepath.Dir(currentPath)
	}

	// Return current directory if a .git directory is present
	if IsSinkProject(currentPath) {
		return currentPath, nil
	}

	// Return user home directory (if the start path was a subpath of it)
	if userHome, err := os.UserHomeDir(); err != nil {
		fmt.Println(err)
	} else if currentPath == userHome {
		return currentPath, errors.New("arrived at user home")
	}

	parentDir := filepath.Join(currentPath, "..")
	// We have to stop at the fs root to prevent infinite recursion
	if parentDir == currentPath {
		return currentPath, errors.New("arrived at fs root")
	}
	return findProjectRoot(parentDir)
}

// Get the (probable) root of a project determined by traversing the filesystem upward from the current working directory ([os.Getwd]).
// If no project can be found, the current working directory is returned.
// If that cannot be retrieved, an error is returned.
//
// See [findProjectRoot] for more information.
func ProjectRoot() (string, error) {
	cwd, err := os.Getwd()
	if err != nil {
		fmt.Println(err)
		return "", err
	}

	projectRoot, err := findProjectRoot(cwd)
	if err != nil {
		fmt.Printf("Failed to find Project root: %s\n", err)
		return cwd, nil
	}
	return projectRoot, nil
}

var WALK_EXEMPT_DIRECTORIES = map[string]bool{
	".git":         true,
	"venv":         true,
	".venv":        true,
	"node_modules": true,
}

// Walks the projectDir to find all *.sink files inside.
//
// Returns a list of paths containing all *.sink files in the project.
// walkSubdir can be used to control subdirectory behaviour.
func FindSinkFiles(projectDir string, walkSubdir bool) []string {
	var foundFiles = make([]string, 0)

	filepath.WalkDir(projectDir, func(path string, d fs.DirEntry, err error) error {
		name := d.Name()
		// fmt.Printf("Walking %s\n", path)

		// Skip directories that we don't want to go walk by default
		if WALK_EXEMPT_DIRECTORIES[name] {
			// fmt.Println("Skipping", name)
			return filepath.SkipDir
		}

		fileInfo, err := d.Info()
		if err != nil {
			return err
		}
		if fileInfo.IsDir() {
			if !walkSubdir && path != projectDir {
				fmt.Println("Skipping", path)
				return filepath.SkipDir
			}
			return nil
		}

		fileExt := filepath.Ext(path)
		if fileExt != ".sink" {
			// fmt.Println("Skipping non-sink file:", path, fileExt)
			return nil
		}
		// fmt.Println("Found .sink file", path)
		foundFiles = append(foundFiles, path)

		return nil
	})

	return foundFiles
}
