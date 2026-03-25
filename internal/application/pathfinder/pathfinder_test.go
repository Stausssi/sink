package pathfinder

import (
	"os"
	"path/filepath"
	"testing"
)

func TestFindGit(t *testing.T) {
	tempDir := t.TempDir()
	tempProjectRoot := filepath.Join(tempDir, "sink")
	os.Mkdir(tempProjectRoot, 0700)
	os.Mkdir(filepath.Join(tempProjectRoot, ".git"), 0400)
	traversalStart := filepath.Join(tempProjectRoot, "some", "nested", "structure", "")
	os.MkdirAll(traversalStart, 0700)
	t.Chdir(traversalStart)

	projectRoot, _ := ProjectRoot()
	if projectRoot != tempProjectRoot {
		t.Errorf("Failed to find .git directory in upwards traversal!\nExpected: %s\nGot: %s", tempProjectRoot, projectRoot)
	}
}

func TestNoProjectStopAtUser(t *testing.T) {
	homeDir, _ := os.UserHomeDir()

	projectRoot, err := findProjectRoot(homeDir)
	if projectRoot != homeDir || err == nil {
		t.Errorf("Failed to stop at home directory!\nExpected: %s\nGot: %s", homeDir, projectRoot)
	}
}

func TestNoProjectStopAtRoot(t *testing.T) {
	traversalStart := t.TempDir()
	t.Chdir(traversalStart)

	projectRoot, _ := ProjectRoot()
	if projectRoot != traversalStart {
		t.Errorf("Didn't return cwd when arriving at fs root!\nExpected: %s\nGot: %s", traversalStart, projectRoot)
	}
}

func TestNonExistentPath(t *testing.T) {
	tempDir := t.TempDir()
	traversalStart := filepath.Join(tempDir, "delete_me")
	os.Mkdir(traversalStart, 0700)
	t.Chdir(traversalStart)
	os.Remove(traversalStart)

	cwd, _ := os.Getwd()
	projectRoot, err := ProjectRoot()
	if err != nil || projectRoot != cwd {
		t.Errorf("Didn't return cwd when starting at a non-existing directory!\nExpected: %s\nGot: %s", traversalStart, projectRoot)
	}
}

func TestFindSinkFiles(t *testing.T) {
	// Create common temp directory
	tempDir := t.TempDir()

	t.Run("TestEmptyDir", func(t *testing.T) {
		files := FindSinkFiles(tempDir, true)
		found := len(files)
		if found > 0 {
			t.Errorf("Expected to find no sink files, found %d", found)
		}
	})

	// Create singular top-level sink file
	rootSinkFile := filepath.Join(tempDir, "dummy.test.sink")
	os.Create(rootSinkFile)
	os.Create(filepath.Join(tempDir, "ignored.md"))

	t.Run("OneFileInRoot", func(t *testing.T) {
		files := FindSinkFiles(tempDir, true)
		found := len(files)
		if found != 1 || files[0] != rootSinkFile {
			t.Errorf("Expected to find one root sink file, found %d (%v)", found, files)
		}
	})

	// Create a .git directory with a .sink file inside
	gitDir := filepath.Join(tempDir, ".git")
	os.Mkdir(gitDir, 0600)
	os.Create(filepath.Join(gitDir, "dummy.test.sink"))

	t.Run("TestExemptDirectory", func(t *testing.T) {
		files := FindSinkFiles(tempDir, true)
		found := len(files)
		if found != 1 || files[0] != rootSinkFile {
			t.Errorf("Expected to find one root sink file, found %d (%v)", found, files)
		}
	})

	// Create nested sink file
	nestedPath := filepath.Join(tempDir, "nested", "structure", ".")
	if err := os.MkdirAll(nestedPath, 0700); err != nil {
		t.Log(err)
	}
	nestedSinkFile := filepath.Join(nestedPath, "dummy.test.sink")
	os.Create(nestedSinkFile)

	t.Run("TestNestedFiles", func(t *testing.T) {
		files := FindSinkFiles(tempDir, true)
		found := len(files)
		if found != 2 || files[0] != rootSinkFile || files[1] != nestedSinkFile {
			t.Errorf("Expected to find two sink files, found %d (%v)", found, files)
		}
	})

	// Nested without subdirectories
	t.Run("TestNestedWithoutSubdirectoryWalk", func(t *testing.T) {
		files := FindSinkFiles(tempDir, false)
		found := len(files)
		if found != 1 || files[0] != rootSinkFile {
			t.Errorf("Expected to find only the root sink file, found %d (%v)", found, files)
		}
	})
}
