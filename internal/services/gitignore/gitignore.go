package gitignore

import (
	"bufio"
	"io"
	"iter"
	"os"
	"strings"
)

const (
	START_PATTERN = "# begin sink-managed section #"
	END_PATTERN   = "# end sink-managed section #"
)

type Gitignore struct {
	linebreak string
	mode      os.FileMode

	beforeSinkBlock []string
	afterSinkBlock  []string
}

// Add the entries to a sink-managed block in the given gitignore.
//
// The sink-managed block is enclosed between [START_PATTERN] and [END_PATTERN]
// Optionally creates the gitignore if it doesn't exist yet.
func AddToGitignore(gitignorePath string, entries []string) error {
	gitignore, err := readGitignore(gitignorePath, true)
	if err != nil {
		return err
	}

	writeGitignore, err := os.OpenFile(gitignorePath, os.O_WRONLY|os.O_CREATE|os.O_TRUNC, gitignore.mode)
	if err != nil {
		return err
	}
	writer := bufio.NewWriter(writeGitignore)
	prevLine := ""

	// First, copy contents before sink block without modification
	for _, line := range gitignore.beforeSinkBlock {
		if _, err := writer.WriteString(line); err != nil {
			return err
		}
		prevLine = line
	}

	// Then, insert sink-managed block
	// with at least one empty line for spacing
	if prevLine != gitignore.linebreak {
		if _, err := writer.WriteString(gitignore.linebreak); err != nil {
			return err
		}
	}
	for line := range sinkBlock(entries) {
		if _, err := writer.WriteString(line + gitignore.linebreak); err != nil {
			return err
		}
	}

	// Followed by original contents that were after the sink-managed block before
	for _, line := range gitignore.afterSinkBlock {
		if _, err := writer.WriteString(line); err != nil {
			return err
		}
	}
	if err := writer.Flush(); err != nil {
		return err
	}
	if err := writeGitignore.Close(); err != nil {
		return err
	}
	return nil
}

func readGitignore(gitignorePath string, createIfNotExists bool) (*Gitignore, error) {
	flag := os.O_RDONLY
	if createIfNotExists {
		flag |= os.O_CREATE
	}
	readGitignore, err := os.OpenFile(gitignorePath, flag, 0644)
	if err != nil {
		return nil, err
	}

	var beforeSinkBlock, afterSinkBlock []string
	linebreakChecked := false
	linebreak := "\n"
	state := 0

	reader := bufio.NewReader(readGitignore)
	for {
		line, err := reader.ReadString('\n')
		if err != nil && err != io.EOF {
			return nil, err
		}

		if !linebreakChecked && line != "" {
			linebreakChecked = true
			if line[len(line)-1] == '\r' {
				linebreak = "\r\n"
			}
		}

		switch strings.Trim(line, " \t\r\n") {
		case START_PATTERN:
			state = 1
		case END_PATTERN:
			state = 2
		default:
			switch state {
			case 0:
				beforeSinkBlock = append(beforeSinkBlock, line)
			case 1:
			case 2:
				afterSinkBlock = append(afterSinkBlock, line)
			}
		}

		if err == io.EOF {
			break
		}
	}
	fileInfo, err := os.Stat(gitignorePath)
	if err != nil {
		return nil, err
	}
	origFileMode := fileInfo.Mode()
	if err := readGitignore.Close(); err != nil {
		return nil, err
	}

	return &Gitignore{
		linebreak:       linebreak,
		mode:            origFileMode,
		beforeSinkBlock: beforeSinkBlock,
		afterSinkBlock:  afterSinkBlock,
	}, nil
}

func sinkBlock(entries []string) iter.Seq[string] {
	return func(yield func(string) bool) {
		if !yield(START_PATTERN) {
			return
		}
		for _, entry := range entries {
			if !yield(entry) {
				return
			}
		}
		if !yield(END_PATTERN) {
			return
		}
	}
}
