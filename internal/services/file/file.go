package file

import (
	"crypto/sha256"
	"errors"
	"fmt"
	"io"
	"io/fs"
	"os"
)

// Create a new file and read the contents of reader into it.
//
// Optionally verifies the given digest.
func ReadIntoFile(reader io.ReadCloser, into string, permissions fs.FileMode, digest *string) error {
	bytes, err := readBytes(reader)
	if err != nil {
		return err
	}

	if digest != nil && *digest != "" {
		if computedDigest := fmt.Sprintf("sha256:%x", sha256.Sum256(bytes)); computedDigest != *digest {
			fmt.Printf("Want: %s, Got: %s\n", *digest, computedDigest)
			return errors.New("computed asset digest is different from expected digest")
		}
	}

	return writeBytesToFile(bytes, into, permissions)
}

func writeBytesToFile(bytes []byte, into string, permissions fs.FileMode) error {
	if err := os.WriteFile(into, bytes, permissions); err != nil {
		return err
	}
	if err := os.Chmod(into, permissions); err != nil {
		return err
	}

	return nil
}

func readBytes(reader io.ReadCloser) ([]byte, error) {
	// TODO: Buffering and all that stuff
	bytes, err := io.ReadAll(reader)
	if err != nil {
		return nil, err
	}
	if err := reader.Close(); err != nil {
		return nil, err
	}
	return bytes, nil
}
