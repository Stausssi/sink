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
// Determined by checking for a .git directory or sink.toml file
func IsSinkProject(path string) bool {
	if gitDir, err := os.Stat(filepath.Join(path, ".git")); err == nil && gitDir.IsDir() {
		return true
	}
	if _, err := os.Stat(filepath.Join(path, "sink.toml")); err == nil {
		return true
	}
	return false
}

func ensureDir(currentPath string) (dirPath string, err error) {
	pathInfo, err := os.Stat(currentPath)
	if err != nil {
		return "", err
	}
	if !pathInfo.IsDir() {
		return filepath.Dir(currentPath), nil
	}
	return currentPath, nil
}

// Traverses upwards through the filesystem starting at currentPath to find the project root directory (see [IsSinkProject]).
//
// Stops at the user home folder and ultimately at the fs root with both cases returning an error.
// Also returns an error if any of the file operations fail, e.g. due to permission issues or non-existent files.
func findProjectRoot(currentDir string) (fsRoot string, isProject bool, err error) {
	// Path sanitation first to remove any links or invalid paths
	currentDir, err = filepath.Abs(currentDir)
	if err != nil {
		return "", false, err
	}
	currentDir, err = ensureDir(currentDir)
	if err != nil {
		return "", false, err
	}

	// Return current directory if a .git directory is present
	if isProject = IsSinkProject(currentDir); isProject {
		return currentDir, isProject, nil
	}

	// Return user home directory (if the start path was a subpath of it)
	if userHome, err := os.UserHomeDir(); err != nil {
		fmt.Println(err)
	} else if currentDir == userHome {
		return currentDir, false, errors.New("arrived at user home")
	}

	parentDir := filepath.Join(currentDir, "..")
	// We have to stop at the fs root to prevent infinite recursion
	if parentDir == currentDir {
		return currentDir, false, errors.New("arrived at fs root")
	}
	return findProjectRoot(parentDir)
}

// Get the (probable) root of a project determined by traversing the filesystem upward from the current working directory ([os.Getwd]).
// If no project can be found, the current working directory is returned.
// If that cannot be retrieved, an error is returned.
//
// See [findProjectRoot] for more information.
func ProjectRoot() (fsRoot string, isProject bool, err error) {
	cwd, err := os.Getwd()
	if err != nil {
		fmt.Println(err)
		return "", false, err
	}

	fsRoot, isProject, err = findProjectRoot(cwd)
	if err != nil {
		return cwd, isProject, nil
	}
	return fsRoot, isProject, nil
}

// Directories to not walk into, based on their name
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
func FindSinkFiles(projectRoot string, walkSubdir bool) []string {
	var foundFiles = make([]string, 0)

	walkErr := filepath.WalkDir(projectRoot, func(path string, d fs.DirEntry, err error) error {
		name := d.Name()
		// fmt.Printf("Walking %s\n", path)

		// Skip directories that we don't want to go walk by default
		if WALK_EXEMPT_DIRECTORIES[name] {
			// fmt.Println("Skipping", name)
			return filepath.SkipDir
		}

		fileInfo, e := d.Info()
		if e != nil {
			return e
		}
		if fileInfo.IsDir() {
			if !walkSubdir && path != projectRoot {
				// fmt.Println("Skipping", path)
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
	if walkErr != nil {
		// fmt.Println("failed to find sink files:", walkErr)
		return nil
	}

	return foundFiles
}

type ProjectGitignoreNotFoundError struct{}

func (e *ProjectGitignoreNotFoundError) Error() string {
	return "no existing gitignore found in project"
}

// Recursively searches the currentDir and upwards for a .gitignore file.
// Stops at the sink project or filesystem root.
//
// In the special case, that a sink project is found but no .gitignore, it will return both gitignorePath and [ProjectGitignoreNotFoundError].
func FindGitignore(currentDir string) (gitignorePath string, err error) {
	currentDir, err = ensureDir(currentDir)
	if err != nil {
		return "", err
	}

	gitignorePath = filepath.Join(currentDir, ".gitignore")
	if _, err := os.Stat(gitignorePath); err == nil {
		return gitignorePath, nil
	}

	if IsSinkProject(currentDir) {
		return gitignorePath, &ProjectGitignoreNotFoundError{}
	}
	parentDir := filepath.Join(currentDir, "..")
	// We have to stop at the fs root to prevent infinite recursion
	if parentDir == currentDir {
		return currentDir, errors.New("arrived at fs root")
	}
	return FindGitignore(parentDir)
}
