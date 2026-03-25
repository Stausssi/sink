package pathfinder

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestFindGit(t *testing.T) {
	tempDir := t.TempDir()
	tempProjectRoot := filepath.Join(tempDir, "sink")
	os.Mkdir(tempProjectRoot, 0700)
	os.Mkdir(filepath.Join(tempProjectRoot, ".git"), 0400)
	traversalStart := filepath.Join(tempProjectRoot, "some", "nested", "structure", "")
	os.MkdirAll(traversalStart, 0700)
	t.Chdir(traversalStart)

	projectRoot, isProject, _ := ProjectRoot()
	if projectRoot != tempProjectRoot {
		t.Errorf("Failed to find .git directory in upwards traversal!\nExpected: %s\nGot: %s", tempProjectRoot, projectRoot)
	}
	assert.True(t, isProject)
}

func TestFindProjectTOML(t *testing.T) {
	tempDir := t.TempDir()
	tempGitRoot := filepath.Join(tempDir, "sink")
	os.Mkdir(tempGitRoot, 0700)
	os.Mkdir(filepath.Join(tempGitRoot, ".git"), 0400)
	tempProjectRoot := filepath.Join(tempGitRoot, "project")
	traversalStart := filepath.Join(tempProjectRoot, "nested", "structure", "")
	os.MkdirAll(traversalStart, 0700)
	os.WriteFile(filepath.Join(tempProjectRoot, "sink.toml"), make([]byte, 0), 0400)
	t.Chdir(traversalStart)

	projectRoot, isProject, _ := ProjectRoot()
	if projectRoot != tempProjectRoot {
		t.Errorf("Failed to find sink.toml in upwards traversal!\nExpected: %s\nGot: %s", tempGitRoot, projectRoot)
	}
	assert.True(t, isProject)
}

func TestNoProjectStopAtUser(t *testing.T) {
	homeDir, _ := os.UserHomeDir()

	projectRoot, isProject, err := findProjectRoot(homeDir)
	if projectRoot != homeDir || err == nil {
		t.Errorf("Failed to stop at home directory!\nExpected: %s\nGot: %s", homeDir, projectRoot)
	}
	assert.False(t, isProject)
}

func TestNoProjectStopAtRoot(t *testing.T) {
	traversalStart := t.TempDir()
	t.Chdir(traversalStart)

	projectRoot, isProject, _ := ProjectRoot()
	if projectRoot != traversalStart {
		t.Errorf("Didn't return cwd when arriving at fs root!\nExpected: %s\nGot: %s", traversalStart, projectRoot)
	}
	assert.False(t, isProject)
}

func TestFindSinkFiles(t *testing.T) {
	// Create common temp directory
	tempDir := t.TempDir()
	fileHandles := make([]*os.File, 0)

	t.Run("TestEmptyDir", func(t *testing.T) {
		files := FindSinkFiles(tempDir, true)
		found := len(files)
		if found > 0 {
			t.Errorf("Expected to find no sink files, found %d", found)
		}
	})

	// Create singular top-level sink file
	rootSinkFile := filepath.Join(tempDir, "dummy.test.sink")
	handle, err := os.Create(rootSinkFile)
	require.Nil(t, err)
	fileHandles = append(fileHandles, handle)
	handle, err = os.Create(filepath.Join(tempDir, "ignored.md"))
	require.Nil(t, err)
	fileHandles = append(fileHandles, handle)

	t.Run("OneFileInRoot", func(t *testing.T) {
		files := FindSinkFiles(tempDir, true)
		found := len(files)
		if found != 1 || files[0] != rootSinkFile {
			t.Errorf("Expected to find one root sink file, found %d (%v)", found, files)
		}
	})

	// Create a .git directory with a .sink file inside
	gitDir := filepath.Join(tempDir, ".git")
	os.Mkdir(gitDir, 0700)
	handle, err = os.Create(filepath.Join(gitDir, "dummy.test.sink"))
	require.Nil(t, err)
	fileHandles = append(fileHandles, handle)

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
	handle, err = os.Create(nestedSinkFile)
	require.Nil(t, err)
	fileHandles = append(fileHandles, handle)

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

	for _, handle := range fileHandles {
		handle.Close()
	}
}

func TestFindGitignore(t *testing.T) {
	tempDir := t.TempDir()
	tempProjectRoot := filepath.Join(tempDir, "sink")
	os.Mkdir(tempProjectRoot, 0700)
	os.Mkdir(filepath.Join(tempProjectRoot, ".git"), 0400)
	gitignorePath := filepath.Join(tempProjectRoot, ".gitignore")
	os.WriteFile(gitignorePath, make([]byte, 0), 0400)

	traversalStart := filepath.Join(tempProjectRoot, "some", "nested", "structure", "")
	os.MkdirAll(traversalStart, 0700)

	found, err := FindGitignore(traversalStart)
	require.Nil(t, err)
	assert.Equal(t, gitignorePath, found)
}

func TestFindNoGitignore(t *testing.T) {
	tempDir := t.TempDir()
	tempProjectRoot := filepath.Join(tempDir, "sink")
	os.Mkdir(tempProjectRoot, 0700)
	os.Mkdir(filepath.Join(tempProjectRoot, ".git"), 0400)

	traversalStart := filepath.Join(tempProjectRoot, "some", "nested", "structure", "")
	os.MkdirAll(traversalStart, 0700)

	found, err := FindGitignore(traversalStart)
	assert.ErrorIs(t, err, &ProjectGitignoreNotFoundError{})
	assert.Equal(t, filepath.Join(tempProjectRoot, ".gitignore"), found)
}
