package github

import "fmt"

func DownloadRelease(owner string, repository string, version string, asset string, into string) error {
	fmt.Printf("Downloading %s from release %s of %s/%s into %s\n", asset, version, owner, repository, into)
	return nil
}
