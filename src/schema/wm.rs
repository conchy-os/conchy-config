pub fn default_wm_kdl() -> &'static str {
    r##"// ==============================================================================
// Conchy OS - Window Manager & Compositor Configuration
// Location: ~/.config/conchy/wm.kdl or /etc/conchy/wm.kdl
// ==============================================================================

// Output / Display configuration
output "eDP-1" {
    scale 1.0
}

// Layout and gaps
layout {
    gaps 16
    center-focused-column "never"

    default-column-width {
        proportion 0.5
    }

    focus-ring {
        width 2
        active-color "#7aa2f7"
        inactive-color "#24283b"
    }

    border {
        off
        width 1
        active-color "#bb9af7"
        inactive-color "#1a1b26"
    }
}

// Visual animations
animations {
    slowdown 1.0
}

// Keybindings
binds {
    Mod+Return { spawn "alacritty"; }
    Mod+Space  { spawn "conchy-menu"; }
    Mod+Q      { close-window; }
    Mod+F      { maximize-column; }
    Print      { spawn "conchy-screenshot"; }

    // Column and window navigation
    Mod+Left   { focus-column-left; }
    Mod+Right  { focus-column-right; }
    Mod+Up     { focus-window-up; }
    Mod+Down   { focus-window-down; }

    Mod+Shift+Left  { move-column-left; }
    Mod+Shift+Right { move-column-right; }

    // Workspaces
    Mod+1 { focus-workspace 1; }
    Mod+2 { focus-workspace 2; }
    Mod+3 { focus-workspace 3; }
    Mod+4 { focus-workspace 4; }

    Mod+Shift+1 { move-column-to-workspace 1; }
    Mod+Shift+2 { move-column-to-workspace 2; }
    Mod+Shift+3 { move-column-to-workspace 3; }
    Mod+Shift+4 { move-column-to-workspace 4; }

    // System logout
    Mod+Shift+E { quit; }
}
"##
}
