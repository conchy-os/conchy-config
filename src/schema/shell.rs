pub fn default_shell_kdl() -> &'static str {
    r#"// ==============================================================================
// Conchy OS - Desktop Shell Configuration
// Location: ~/.config/conchy/shell.kdl or /etc/conchy/shell.kdl
// ==============================================================================

panel {
    position "top"
    height 32
    opacity 0.95
    font "Inter 10"

    // Theme: dark, light, or auto
    theme "dark"

    // Enabled widgets from left to right
    left {
        widget "workspaces"
        widget "window-title"
    }

    center {
        widget "clock" {
            format "%a %d %b %H:%M"
        }
    }

    right {
        widget "tray"
        widget "volume"
        widget "battery"
        widget "power-menu"
    }
}
"#
}
