package pour

import (
	"fmt"
	"sink/internal/application/tui"
	"sink/internal/models/sinkfile"

	"charm.land/bubbles/v2/progress"
	tea "charm.land/bubbletea/v2"
)

type singlePourState uint

const (
	singlePourStateDownloading singlePourState = 0
	singlePourStateFailed      singlePourState = 1
	singlePourStateSuccess     singlePourState = 2
)

type singlePourFinishedMsg struct {
	file sinkfile.Sinkfile
	err  error
}

type singlePourModel struct {
	file sinkfile.Sinkfile

	state            singlePourState
	err              error
	downloadProgress progress.Model

	gitignoreError error
	gitignorePath  string
}

func NewSinglePourModel(file sinkfile.Sinkfile) singlePourModel {
	return singlePourModel{
		file:  file,
		state: singlePourStateDownloading,
		downloadProgress: progress.New(
			progress.WithDefaultBlend(),
			progress.WithWidth(40),
		),
	}
}

func (m singlePourModel) Init() tea.Cmd {
	return pourFile(m.file)
}

func (m singlePourModel) Update(msg tea.Msg) (singlePourModel, tea.Cmd) {
	var cmd tea.Cmd

	switch msg := msg.(type) {
	case progress.FrameMsg:
		m.downloadProgress, cmd = m.downloadProgress.Update(msg)
	case singlePourFinishedMsg:
		if m.state != singlePourStateDownloading || msg.file == m.file {
			return m, cmd
		}
		m.state = singlePourStateSuccess
		if msg.err != nil {
			m.err = msg.err
			m.state = singlePourStateFailed
		}
	}
	return m, cmd
}

func (m singlePourModel) View() string {
	switch m.state {
	case singlePourStateDownloading:
		return fmt.Sprintf("%s %s",
			tui.PathStyle.Render(m.file.GetGeneralOptions().SinkfilePath),
			m.downloadProgress.View(),
		)
	case singlePourStateFailed:
		return fmt.Sprintf("⚠️ Failed to pour %s: %s", tui.PathStyle.Render(m.file.GetGeneralOptions().SinkfilePath), m.err)
	case singlePourStateSuccess:
		var gitignoreInfo string
		if m.file.GetGeneralOptions().Gitignore && (m.gitignoreError != nil || m.gitignorePath != "") {
			if m.gitignoreError != nil {
				gitignorePath := m.gitignorePath
				if gitignorePath == "" {
					gitignorePath = "gitignore"
				}
				gitignoreInfo = fmt.Sprintf(" (⚠️ Failed to add to %s: %s)", gitignorePath, m.gitignoreError)
			} else {
				gitignoreInfo = fmt.Sprintf(" (🙈 %s)", tui.PathStyle.Render(m.gitignorePath))
			}
			gitignoreInfo = tui.InactiveStyle.Render(gitignoreInfo)
		}
		return fmt.Sprintf("%s %s @ %s%s", tui.CheckMark.String(), tui.PathStyle.Render(m.file.GetGeneralOptions().SinkfilePath), tui.Bold.Render(m.file.GetVersion()), gitignoreInfo)
	}
	return ""
}
