package project

import (
	"errors"
	"fmt"
	"io/fs"
	"path/filepath"
	"sink/internal/application/tui"
	"sink/internal/models/config"
	"sink/internal/services/pathfinder"

	"charm.land/bubbles/v2/spinner"
	tea "charm.land/bubbletea/v2"
)

type ProjectRootModel struct {
	Config config.Config
	Fatal  error

	failableSearch tui.FailableActivityModel
}

func NewProjectRootModel() ProjectRootModel {
	return ProjectRootModel{
		failableSearch: tui.NewFailableActivity(newSearchModel(), "Failed to identify project root", nil),
	}
}

func (m ProjectRootModel) Init() tea.Cmd {
	return m.failableSearch.Init()
}

func (m ProjectRootModel) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmd tea.Cmd

	switch msg := msg.(type) {
	case tea.KeyPressMsg:
		if msg.Mod == tea.ModCtrl && msg.Code == 'c' {
			return m, tea.Quit
		}
	case searchFinishedMsg:
		m.failableSearch.Finished = true
		m.Config = msg.config
		if msg.err != nil {
			// If no project root was found, it's a fatal error
			// Otherwise, it's just a warning to display in the failableActivity
			if msg.config.ProjectRoot == "" {
				m.Fatal = msg.err
			}
			m.failableSearch.Failure = msg.err
		}
		return m, tea.Quit
	}

	m.failableSearch, cmd = m.failableSearch.Update(msg)
	return m, cmd
}

func (m ProjectRootModel) View() tea.View {
	var viewString string
	if !m.failableSearch.Finished || m.failableSearch.Failure != nil {
		viewString = m.failableSearch.View()
	} else if m.Config.IsProject {
		viewString = fmt.Sprintf("Project rooted at %s\n", tui.InactiveStyle.Inherit(tui.PathStyle).Render(m.Config.ProjectRoot))
	} else {
		viewString = "No sink project found\n"
	}

	return tea.NewView(tui.InactiveStyle.Render(viewString))
}

type searchModel struct {
	spinner spinner.Model
}
type searchFinishedMsg struct {
	config config.Config
	err    error
}

func newSearchModel() searchModel {
	return searchModel{
		spinner: spinner.New(spinner.WithSpinner(spinner.Ellipsis)),
	}
}

func (m searchModel) Init() tea.Cmd {
	return tea.Batch(func() tea.Msg {
		cfg := config.NewConfigWithDefaults()

		determinedRoot, isProject, err := pathfinder.ProjectRoot()
		if err != nil {
			return searchFinishedMsg{config: cfg, err: err}
		}
		cfg.ProjectRoot = determinedRoot
		cfg.IsProject = isProject

		if isProject {
			err = cfg.LoadConfig(filepath.Join(determinedRoot, "sink.toml"))
			if !errors.Is(err, fs.ErrNotExist) {
				return searchFinishedMsg{config: cfg, err: err}
			}
		}

		return searchFinishedMsg{config: cfg}
	}, m.spinner.Tick)
}

func (m searchModel) Update(msg tea.Msg) (tui.ChildModel, tea.Cmd) {
	var cmd tea.Cmd

	switch msg := msg.(type) {
	case spinner.TickMsg:
		m.spinner, cmd = m.spinner.Update(msg)
	}
	return m, cmd
}
func (m searchModel) View() string {
	return fmt.Sprintf("Identifying project root %s", m.spinner.View())
}
