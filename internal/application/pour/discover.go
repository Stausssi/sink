package pour

import (
	"fmt"
	"sink/internal/application/tui"
	"sink/internal/models/config"
	"sink/internal/models/sinkfile"
	"sink/internal/services/pathfinder"
	"strconv"
	"strings"

	"charm.land/bubbles/v2/spinner"
	tea "charm.land/bubbletea/v2"
)

type discoveredFilesMsg struct {
	files  []sinkfile.Sinkfile
	issues map[string]error
}

type discoverModel struct {
	config config.Config

	discoveredFiles []sinkfile.Sinkfile
	nFiles          int
	discoveryIssues map[string]error
	nIssues         int
	finished        bool

	spinner spinner.Model
}

func NewDiscoverModel(config config.Config) discoverModel {
	return discoverModel{
		config:  config,
		spinner: spinner.New(spinner.WithSpinner(spinner.Ellipsis)),
	}
}

func discoverFiles(config config.Config) tea.Cmd {
	return func() tea.Msg {
		// For testing animations
		// time.Sleep(time.Second * 2)

		// TODO: Error handling of FindSinkFiles
		candidates := pathfinder.FindSinkFiles(config.ProjectRoot, config.IsProject)

		issues := make(map[string]error, 0)
		files := make([]sinkfile.Sinkfile, 0, len(candidates))

		for _, path := range candidates {
			parsedFile, err := sinkfile.ParseSinkfile(config, path)
			if err != nil {
				issues[path] = err
				continue
			}
			files = append(files, parsedFile)
		}

		return discoveredFilesMsg{
			files:  files,
			issues: issues,
		}
	}
}

func (m discoverModel) Init() tea.Cmd {
	return tea.Batch(discoverFiles(m.config), m.spinner.Tick)
}

func (m discoverModel) Update(msg tea.Msg) (discoverModel, tea.Cmd) {
	var cmd tea.Cmd
	switch msg := msg.(type) {
	case spinner.TickMsg:
		m.spinner, cmd = m.spinner.Update(msg)
	case discoveredFilesMsg:
		m.discoveredFiles = msg.files
		m.nFiles = len(msg.files)
		m.discoveryIssues = msg.issues
		m.nIssues = len(msg.issues)
		m.finished = true

		if m.nFiles <= 0 && m.nIssues <= 0 {
			return m, func() tea.Msg { return tui.ExitMsg(0) }
		}
		if m.nFiles-m.nIssues <= 0 {
			return m, func() tea.Msg { return tui.ExitMsg(1) }
		}
	}

	return m, cmd
}

func (m discoverModel) View() string {
	if !m.finished {
		return fmt.Sprintf("🔎 Discovering sinkfiles %s", m.spinner.View())
	}

	if m.nFiles <= 0 && m.nIssues <= 0 {
		return "No files to sink\n"
	}

	var view strings.Builder
	fmt.Fprintf(&view, "Discovered %s files.", tui.NumberStyle.Render(strconv.Itoa(m.nFiles)))
	for path, issue := range m.discoveryIssues {
		fmt.Fprintf(&view, "\n%s", tui.IndentStyle.Render(fmt.Sprintf("⚠️ Invalid sinkfile at %s: %s", tui.PathStyle.Render(path), issue)))
	}
	view.WriteRune('\n')
	return view.String()
}
