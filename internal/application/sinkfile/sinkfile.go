package sinkfile

import (
	"errors"
	"fmt"
	"net/url"
	config "sink/internal"
	"sink/internal/services/git"
	"sink/internal/services/github"
	"strings"

	"io/fs"

	"gopkg.in/ini.v1"
)

type Sinkfile interface {
	Validate() error
	Get(into string) error
	GetGeneralOptions() GeneralOptions
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

	// --------------------
	// NON-INI FIELDS BELOW
	// --------------------

	// Path to the sinkfile in the local filesystem
	SinkfilePath string `ini:"-"`
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
	return git.GetFileFromRepository(g.GitRepoUrl, g.Ref, g.FilePathInRepo, into)
}
func (g *GitBasedSinkfile) GetGeneralOptions() GeneralOptions {
	return g.GeneralOptions
}

type ReleaseBasedSinkfile struct {
	GeneralOptions `ini:"DEFAULT"`

	Source   string `ini:"source"`
	Version  string `ini:"version"`
	Checksum string `ini:"checksum"`

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
	return github.DownloadRelease(r.SourceRepository.Owner, r.SourceRepository.RepositoryName, r.Version, r.AssetName, into)
}
func (r *ReleaseBasedSinkfile) GetGeneralOptions() GeneralOptions {
	return r.GeneralOptions
}

func ParseSinkfile(config config.Config, path string) (Sinkfile, error) {
	iniContents, err := ini.ShadowLoad(path)
	if err != nil {
		return nil, err
	}

	generalOptions := GeneralOptions{
		Gitignore:    config.Defaults.Gitignore,
		Lock:         config.Defaults.Lock,
		SinkfilePath: path,
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
