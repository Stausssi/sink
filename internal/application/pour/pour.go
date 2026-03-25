package pour

import (
	"fmt"
	"path/filepath"
	"sink/internal/application/tui"
	"sink/internal/models/config"
	"sink/internal/models/sinkfile"
	"sink/internal/services/gitignore"
	"sink/internal/services/pathfinder"
	"strconv"
	"strings"

	"charm.land/bubbles/v2/spinner"
	tea "charm.land/bubbletea/v2"
)

type gitignoreManipulationFinishedMsg struct {
	gitignore string
	err       error
}

func pourFile(file sinkfile.Sinkfile) tea.Cmd {
	return func() tea.Msg {
		err := file.Get(file.GetGeneralOptions().TargetPath)
		return singlePourFinishedMsg{
			file: file,
			err:  err,
		}
	}
}

func manipulateGitignore(gitignoreEntries map[string][]string) tea.Cmd {
	cmds := make([]tea.Cmd, 0, len(gitignoreEntries))
	for file, entries := range gitignoreEntries {
		cmds = append(cmds, func() tea.Msg {
			msg := gitignoreManipulationFinishedMsg{
				gitignore: file,
			}
			if err := gitignore.AddToGitignore(file, entries); err != nil {
				msg.err = err
			}
			return msg
		})
	}
	return tea.Batch(cmds...)
}

type PourModel struct {
	config config.Config

	ExitCode int

	discoverModel discoverModel

	singlePours      map[sinkfile.Sinkfile]*singlePourModel
	pourSpinner      spinner.Model
	nPours           int
	nFinishedPours   int
	nSuccessfulPours int
	allPoursDone     bool

	gitignoreEntries map[string][]sinkfile.Sinkfile
	noGitignoreFound bool
}

func NewPourModel(config config.Config) PourModel {
	return PourModel{
		config:           config,
		discoverModel:    NewDiscoverModel(config),
		singlePours:      make(map[sinkfile.Sinkfile]*singlePourModel),
		pourSpinner:      spinner.New(spinner.WithSpinner(spinner.Ellipsis)),
		gitignoreEntries: make(map[string][]sinkfile.Sinkfile),
	}
}

func (m PourModel) Init() tea.Cmd {
	return m.discoverModel.Init()
}

func (m PourModel) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmd tea.Cmd

	cmds := make([]tea.Cmd, 0)
	if !m.discoverModel.finished {
		m.discoverModel, cmd = m.discoverModel.Update(msg)
		cmds = append(cmds, cmd)
	}
	for file, pourActivity := range m.singlePours {
		updatedModel, cmd := pourActivity.Update(msg)
		m.singlePours[file] = &updatedModel
		cmds = append(cmds, cmd)
	}

	switch msg := msg.(type) {
	case tea.KeyPressMsg:
		if msg.Mod == tea.ModCtrl && msg.Code == 'c' {
			return m, tea.Quit
		}
	case tui.ExitMsg:
		m.ExitCode = int(msg)
		return m, tea.Quit
	case spinner.TickMsg:
		m.pourSpinner, cmd = m.pourSpinner.Update(msg)
		cmds = append(cmds, cmd)
	case discoveredFilesMsg:
		cmds = append(cmds, m.pourSpinner.Tick)
		for _, file := range msg.files {
			pourModel := NewSinglePourModel(file)
			m.singlePours[file] = &pourModel
			m.nPours++
			cmds = append(cmds, pourModel.Init())
		}
	case singlePourFinishedMsg:
		m.nFinishedPours++
		if msg.err == nil {
			m.nSuccessfulPours++
		}
		wantsGitignore := m.config.IsProject && !m.noGitignoreFound && msg.file.GetGeneralOptions().Gitignore
		if wantsGitignore {
			nearestGitignore, err := pathfinder.FindGitignore(msg.file.GetGeneralOptions().TargetPath)
			if err != nil {
				m.noGitignoreFound = true
				break
			}

			if m.gitignoreEntries[nearestGitignore] == nil {
				m.gitignoreEntries[nearestGitignore] = make([]sinkfile.Sinkfile, 0, 1)
			}
			m.gitignoreEntries[nearestGitignore] = append(m.gitignoreEntries[nearestGitignore], msg.file)
		}

		m.allPoursDone = m.nFinishedPours >= m.nPours
		if !m.allPoursDone {
			break
		}
		if !m.config.IsProject || m.noGitignoreFound {
			cmds = append(cmds, func() tea.Msg { return tui.ExitMsg(0) })
			break
		}

		relativeEntries := make(map[string][]string)
		for gitignore, files := range m.gitignoreEntries {
			entries := make([]string, 0, len(files))
			for _, file := range files {
				relativeTarget, _ := filepath.Rel(filepath.Dir(gitignore), file.GetGeneralOptions().TargetPath)
				entries = append(entries, relativeTarget)
			}
			relativeEntries[gitignore] = entries
		}

		cmds = append(cmds, manipulateGitignore(relativeEntries))
	case gitignoreManipulationFinishedMsg:
		for _, file := range m.gitignoreEntries[msg.gitignore] {
			if !file.GetGeneralOptions().Gitignore {
				continue
			}

			m.singlePours[file].gitignorePath = msg.gitignore
			m.singlePours[file].gitignoreError = msg.err
		}
		delete(m.gitignoreEntries, msg.gitignore)

		if len(m.gitignoreEntries) == 0 {
			cmds = append(cmds, func() tea.Msg { return tui.ExitMsg(0) })
		}
	}

	return m, tea.Batch(cmds...)
}

func (m PourModel) View() tea.View {
	var viewBuilder strings.Builder

	if !m.discoverModel.finished || m.discoverModel.nIssues > 0 {
		viewBuilder.WriteString(m.discoverModel.View())
	}

	if m.discoverModel.finished {
		if !m.allPoursDone {
			fmt.Fprintf(&viewBuilder, "🌧️ Pouring %s\n", m.pourSpinner.View())
		} else {
			sun := "⛅"
			if m.nSuccessfulPours == m.nPours {
				sun = "🌤️"
			}
			fmt.Fprintf(&viewBuilder, "%s Poured %s files:",
				sun,
				tui.NumberStyle.Render(strconv.Itoa(m.nSuccessfulPours)))
			viewBuilder.WriteRune('\n')
		}

		for _, pourActivity := range m.singlePours {
			viewBuilder.WriteString(tui.IndentStyle.Render(pourActivity.View()))
			viewBuilder.WriteRune('\n')
		}
	}

	if m.config.IsProject && m.allPoursDone && m.noGitignoreFound {
		viewBuilder.WriteString(tui.InactiveStyle.Render(
			"💡 No gitignore found.",
			"Please create one to automatically ignore poured files.",
		))
		viewBuilder.WriteRune('\n')
	}

	return tea.NewView(viewBuilder.String())
}
