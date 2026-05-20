package tui

import "charm.land/lipgloss/v2"

var (
	Bold          = lipgloss.NewStyle().Bold(true)
	IndentStyle   = lipgloss.NewStyle().Margin(0, 2)
	InactiveStyle = lipgloss.NewStyle().Faint(true)
	WarnStyle     = lipgloss.NewStyle().Foreground(lipgloss.Yellow)
	PathStyle     = lipgloss.NewStyle().Italic(true)
	NumberStyle   = lipgloss.NewStyle().Inherit(Bold)
	CheckMark     = lipgloss.NewStyle().Foreground(lipgloss.Color("42")).SetString("✔︎")
)
