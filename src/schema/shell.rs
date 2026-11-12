pub fn default_shell_kdl() -> &'static str {
    r#"// ==============================================================================
// Conchy OS - Desktop Shell Configuration
// Location: ~/.config/conchy/shell.kdl
//
// The shell's settings window writes this file one node at a time, so what you
// write here, comments included, stays as you wrote it. Anything left out takes
// its default. The full spelling is in conchy-shell/docs/CONFIG_SPEC.md.
// ==============================================================================

bar {
    position "top"      // "top", "bottom", "left" or "right"
    style "floating"
    size 32.0
}
"#
}
