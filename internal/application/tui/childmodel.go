package tui

import tea "charm.land/bubbletea/v2"

type ChildModel interface {
	Init() tea.Cmd
	Update(msg tea.Msg) (ChildModel, tea.Cmd)
	View() string
}
