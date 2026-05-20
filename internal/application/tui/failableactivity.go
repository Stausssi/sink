package tui

import (
	"fmt"

	tea "charm.land/bubbletea/v2"
	"charm.land/lipgloss/v2"
)

// Simple utility model that renders a child activity that can fail.
type FailableActivityModel struct {
	Finished      bool
	Failure       error
	FailurePrefix string
	FailureStyle  *lipgloss.Style

	ChildModel ChildModel
}

func NewFailableActivity(child ChildModel, failurePrefix string, failureStyle *lipgloss.Style) FailableActivityModel {
	return FailableActivityModel{
		FailurePrefix: failurePrefix,
		FailureStyle:  failureStyle,
		ChildModel:    child,
	}
}

func (m FailableActivityModel) Init() tea.Cmd {
	return m.ChildModel.Init()
}

func (m FailableActivityModel) Update(msg tea.Msg) (FailableActivityModel, tea.Cmd) {
	var cmd tea.Cmd

	m.ChildModel, cmd = m.ChildModel.Update(msg)

	return m, cmd
}

func (m FailableActivityModel) View() string {
	if !m.Finished {
		return m.ChildModel.View()
	}
	if m.Failure != nil {
		failureString := fmt.Sprintf("⚠️ %s: %s", m.FailurePrefix, m.Failure)
		if m.FailureStyle != nil {
			failureString = m.FailureStyle.Render(failureString)
		}
		return failureString
	}
	return ""
}
