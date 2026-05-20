package sinkfile

import (
	"errors"
	"fmt"
	"net/url"
	"path/filepath"
	"sink/internal/models/config"
	"sink/internal/services/github"
	"strings"

	"io/fs"

	"gopkg.in/ini.v1"
)

type Sinkfile interface {
	Validate() error
	Get(into string) error
	GetGeneralOptions() GeneralOptions
	GetVersion() string
}

// Options that apply to every sinkfile, regardless of git- or release-based.
type GeneralOptions struct {
	// Whether to add this file to the .gitignore
	Gitignore bool `ini:"gitignore"`

	// Whether to create a lockfile entry for this file
	Lock bool `ini:"lock"`

	// Optional list of symlinks to create for this file
	Symlink []string `ini:"symlink,,allowshadow"`

	// Filesystem permissions of the downloaded file
	Permissions fs.FileMode `ini:"permissions"`

	// -------------------- //
	// NON-INI FIELDS BELOW //
	// -------------------- //

	// Path to the sinkfile in the local filesystem
	SinkfilePath string `ini:"-"`

	// Path to the target file in the local filesystem
	TargetPath string `ini:"-"`
}

type GitBasedSinkfile struct {
	GeneralOptions `ini:"DEFAULT"`

	Source string `ini:"source"`
	Ref    string `ini:"ref"`

	// --------------------
	// NON-INI FIELDS BELOW
	// --------------------

	GitRepoUrl     url.URL `ini:"-"`
	FilePathInRepo string  `ini:"-"`
}

func (g *GitBasedSinkfile) Validate() error {
	if g.Source == "" {
		return errors.New("empty source")
	}
	if g.Ref == "" {
		return errors.New("empty ref")
	}

	urlSource, err := url.Parse(g.Source)
	if err != nil {
		return err
	}

	repoUrlPath, file, found := strings.Cut(urlSource.Path, ":")
	if !found || file == "" {
		return errors.New("missing or empty file information")
	}
	g.FilePathInRepo = file

	urlSource.Path = repoUrlPath
	g.GitRepoUrl = *urlSource

	return nil
}
func (g GitBasedSinkfile) Get(into string) error {
	host := g.GitRepoUrl.Hostname()

	if host == "github.com" {
		owner, repo, err := extractOwnerAndRepoFromURL(g.GitRepoUrl)
		if err != nil {
			return err
		}
		return github.DownloadSingleFile(owner, repo, g.Ref, g.FilePathInRepo, into, g.Permissions)
	}

	return fmt.Errorf("downloading from %s is not yet supported", host)
}
func (g *GitBasedSinkfile) GetGeneralOptions() GeneralOptions {
	return g.GeneralOptions
}

func (g *GitBasedSinkfile) GetVersion() string {
	return g.Ref
}

func extractOwnerAndRepoFromURL(url url.URL) (owner string, repository string, err error) {
	for part := range strings.SplitSeq(url.Path, "/") {
		if strings.HasSuffix(part, ".git") {
			repository = part
			break
		}
		owner = part
	}
	if owner == "" {
		return "", "", errors.New("repository url does not contain an owner fragment")
	}
	repository = strings.TrimSuffix(repository, ".git")
	if repository == "" {
		return "", "", errors.New("repository url does not contain a valid repository fragment")
	}

	return owner, repository, nil
}

type ReleaseBasedSinkfile struct {
	GeneralOptions `ini:"DEFAULT"`

	Source  string `ini:"source"`
	Version string `ini:"version"`
	Digest  string `ini:"digest"`

	// --------------------
	// NON-INI FIELDS BELOW
	// --------------------

	SourceRepository struct {
		Owner          string
		RepositoryName string
	}
	AssetName string
}

func (r *ReleaseBasedSinkfile) Validate() error {
	if r.Source == "" {
		return errors.New("empty source")
	}
	if r.Version == "" {
		return errors.New("empty version")
	}

	repository, asset, found := strings.Cut(r.Source, ":")
	if !found || asset == "" {
		return errors.New("missing or empty asset")
	}
	r.AssetName = asset

	owner, repository, found := strings.Cut(repository, "/")
	if !found || owner == "" || repository == "" {
		return errors.New("missing or incomplete repository information")
	}
	r.SourceRepository.Owner = owner
	r.SourceRepository.RepositoryName = repository

	return nil
}
func (r ReleaseBasedSinkfile) Get(into string) error {
	return github.DownloadFromRelease(r.SourceRepository.Owner, r.SourceRepository.RepositoryName, r.Version, r.AssetName, into, r.Permissions, r.Digest)
}
func (r *ReleaseBasedSinkfile) GetGeneralOptions() GeneralOptions {
	return r.GeneralOptions
}
func (r *ReleaseBasedSinkfile) GetVersion() string {
	return r.Version
}

func ParseSinkfile(config config.Config, path string) (Sinkfile, error) {
	iniContents, err := ini.ShadowLoad(path)
	if err != nil {
		return nil, err
	}

	generalOptions := GeneralOptions{
		Gitignore:    config.Defaults.Gitignore,
		Lock:         config.Defaults.Lock,
		Permissions:  0644,
		SinkfilePath: path,
		TargetPath:   strings.TrimSuffix(path, filepath.Ext(path)),
	}
	errors := make([]error, 0, 2)

	gitBased := &GitBasedSinkfile{
		GeneralOptions: generalOptions,
	}
	if err = iniContents.StrictMapTo(&gitBased); err == nil {
		err = gitBased.Validate()
		if err == nil {
			return gitBased, nil
		}
	}
	errors = append(errors, err)

	releaseBased := &ReleaseBasedSinkfile{
		GeneralOptions: generalOptions,
	}
	if err = iniContents.StrictMapTo(&releaseBased); err == nil {
		err = releaseBased.Validate()
		if err == nil {
			return releaseBased, nil
		}
	}
	errors = append(errors, err)

	return nil, fmt.Errorf("failed to parse ini into sinkfile: %s", errors)
}
