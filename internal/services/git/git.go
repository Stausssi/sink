package git

import (
	"fmt"
	"net/url"
)

func GetFileFromRepository(repository url.URL, ref string, path string, into string) error {
	fmt.Printf("Downloading %s from %s@%s into %s\n", path, repository.String(), ref, into)
	return nil
}
