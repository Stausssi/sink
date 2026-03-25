package sinkfile

import (
	"net/url"
	"sink/internal/models/config"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestParseGitBased(t *testing.T) {
	t.Parallel()

	repoUrl, _ := url.Parse("https://github.com/Stausssi/sink")

	gitBased, err := ParseSinkfile(config.NewConfigWithDefaults(), "test/gitbased.sink.ignore")
	assert.Nil(t, err)
	require.NotNil(t, gitBased)

	gitBasedStruct, ok := gitBased.(*GitBasedSinkfile)
	require.True(t, ok)
	assert.EqualValues(t, gitBasedStruct, &GitBasedSinkfile{
		GeneralOptions: GeneralOptions{
			Gitignore:   true,
			Lock:        true,
			Symlink:     []string{"another_file", "even_more"},
			Permissions: 0777,
			// Non-ini fields but filled regardless
			SinkfilePath: "test/gitbased.sink.ignore",
			TargetPath:   "test/gitbased.sink",
		},
		Source: "https://github.com/Stausssi/sink:internal/application/sinkfile/test/gitbased.sink.ignore",
		Ref:    "main",
		// Non-ini fields but filled regardless
		GitRepoUrl:     *repoUrl,
		FilePathInRepo: "internal/application/sinkfile/test/gitbased.sink.ignore",
	})
}

func TestParseReleaseBased(t *testing.T) {
	t.Parallel()

	releaseBased, err := ParseSinkfile(config.NewConfigWithDefaults(), "test/releasebased.sink.ignore")
	assert.Nil(t, err)
	require.NotNil(t, releaseBased)

	releaseBasedStruct, ok := releaseBased.(*ReleaseBasedSinkfile)
	require.True(t, ok)
	assert.EqualValues(t, releaseBasedStruct, &ReleaseBasedSinkfile{
		GeneralOptions: GeneralOptions{
			Gitignore:   true,
			Lock:        true,
			Symlink:     []string{"another_file", "even_more"},
			Permissions: 0777,
			// Non-ini fields but filled regardless
			SinkfilePath: "test/releasebased.sink.ignore",
			TargetPath:   "test/releasebased.sink",
		},
		Source:  "Stausssi/sink:releasebased.sink.ignore",
		Version: "v1.0.0",
		Digest:  "53b9f00bb33bb6b20028831416c6e4143d7950db9ce1e83c3e6ef4164415240c",
		// Non-ini fields but filled regardless
		SourceRepository: struct {
			Owner          string
			RepositoryName string
		}{
			Owner:          "Stausssi",
			RepositoryName: "sink",
		},
		AssetName: "releasebased.sink.ignore",
	})
}
