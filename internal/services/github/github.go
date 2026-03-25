package github

import (
	"context"
	"errors"
	"io/fs"
	"sink/internal/services/file"

	"github.com/cli/go-gh/v2/pkg/auth"
	"github.com/google/go-github/v84/github"
)

var client *github.Client

func ensureClientExistsAndAuthenticated() error {
	if client == nil {
		authToken, _ := auth.TokenForHost("github.com")
		if authToken == "" {
			return errors.New("failed to get auth token from GH CLI backend")
		}
		client = github.NewClient(nil).WithAuthToken(authToken)
	}
	return nil
}

// Download an asset from a GitHub release into a given file with digest validation.
func DownloadFromRelease(owner string, repository string, tag string, assetName string, into string, permissions fs.FileMode, digest string) error {
	if err := ensureClientExistsAndAuthenticated(); err != nil {
		return err
	}

	// fmt.Printf("Downloading %s from release %s of %s/%s into %s\n", assetName, tag, owner, repository, into)

	release, _, err := client.Repositories.GetReleaseByTag(context.Background(), owner, repository, tag)
	if err != nil {
		return err
	}

	var releaseAsset *github.ReleaseAsset
	for _, asset := range release.Assets {
		if *asset.Name == assetName {
			releaseAsset = asset
			break
		}
	}
	if releaseAsset == nil {
		return errors.New("failed to find asset in release")
	}

	// Pre-validate the digest before downloading - if available
	if digest != "" {
		if assetDigest := releaseAsset.GetDigest(); assetDigest != "" {
			if assetDigest != digest {
				// fmt.Printf("Want: %s, Got: %s\n", digest, assetDigest)
				return errors.New("asset digest is different from expected digest")
			}
			// "Reset" digest so that it's not checked twice
			digest = ""
		}
	}

	reader, _, err := client.Repositories.DownloadReleaseAsset(context.Background(), owner, repository, *releaseAsset.ID, client.Client())
	if err != nil {
		return err
	}

	return file.ReadIntoFile(reader, into, permissions, &digest)
}

// Download a single file out of the given repository into the given file using the GitHub API.
func DownloadSingleFile(owner string, repository string, ref string, path string, into string, permissions fs.FileMode) error {
	if err := ensureClientExistsAndAuthenticated(); err != nil {
		return err
	}
	// fmt.Printf("Downloading %s from github.com/%s/%s@%s into %s\n", path, owner, repository, ref, into)

	reader, _, err := client.Repositories.DownloadContents(context.Background(), owner, repository, path, &github.RepositoryContentGetOptions{Ref: ref})
	if err != nil {
		return err
	}

	return file.ReadIntoFile(reader, into, permissions, nil)
}
