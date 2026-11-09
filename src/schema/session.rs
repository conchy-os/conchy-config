use anyhow::{Context, Result};
use kdl::{KdlDocument, KdlValue};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupervisedService {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub restart: String,
    pub wait_for_wayland: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionConfig {
    pub socket_wait_timeout_secs: u64,
    pub autostart: bool,
    pub compositor: String,
    pub compositor_args: Vec<String>,
    pub env: HashMap<String, String>,
    pub services: Vec<SupervisedService>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            socket_wait_timeout_secs: 10,
            autostart: true,
            compositor: "conchy-wm".to_string(),
            compositor_args: Vec::new(),
            env: HashMap::new(),
            services: vec![
                SupervisedService {
                    name: "pipewire".to_string(),
                    command: "pipewire".to_string(),
                    args: Vec::new(),
                    restart: "always".to_string(),
                    wait_for_wayland: false,
                },
                SupervisedService {
                    name: "wireplumber".to_string(),
                    command: "wireplumber".to_string(),
                    args: Vec::new(),
                    restart: "always".to_string(),
                    wait_for_wayland: false,
                },
                SupervisedService {
                    name: "conchy-config".to_string(),
                    command: "conchy-config".to_string(),
                    args: vec!["daemon".to_string()],
                    restart: "always".to_string(),
                    wait_for_wayland: false,
                },
                SupervisedService {
                    name: "conchy-shell".to_string(),
                    command: "conchy-shell".to_string(),
                    args: Vec::new(),
                    restart: "always".to_string(),
                    wait_for_wayland: true,
                },
            ],
        }
    }
}

pub fn default_session_kdl() -> &'static str {
    r#"// ==============================================================================
// Conchy OS - Wayland User Session Configuration
// Location: ~/.config/conchy/session.kdl or /etc/conchy/session.kdl
// ==============================================================================

session {
    socket-wait-timeout 10
    autostart true
}

env {
    XCURSOR_SIZE "24"
}

compositor "conchy-wm"

// ==============================================================================
// Supervised Services (restarted automatically on crash)
// ==============================================================================

service "pipewire" {
    command "pipewire"
    restart "always"
    wait-for-wayland false
}

service "wireplumber" {
    command "wireplumber"
    restart "always"
    wait-for-wayland false
}

service "conchy-config" {
    command "conchy-config"
    args "daemon"
    restart "always"
    wait-for-wayland false
}

service "conchy-shell" {
    command "conchy-shell"
    restart "always"
    wait-for-wayland true
}
"#
}

/// Parses a KdlDocument into a strongly-typed SessionConfig
pub fn parse_session_config(doc: &KdlDocument) -> Result<SessionConfig> {
    let mut config = SessionConfig::default();

    // Parse session { ... } block
    if let Some(session_node) = doc.get("session") {
        if let Some(children) = session_node.children() {
            if let Some(KdlValue::Base10(timeout)) = children.get_arg("socket-wait-timeout") {
                config.socket_wait_timeout_secs = *timeout as u64;
            }
            if let Some(KdlValue::Bool(autostart)) = children.get_arg("autostart") {
                config.autostart = *autostart;
            }
        }
    }

    // Parse compositor
    if let Some(node) = doc.get("compositor") {
        if let Some(first) = node.entries().first() {
            if let KdlValue::String(cmd) = first.value() {
                config.compositor = cmd.clone();
            }
        }
    }

    // Parse env { ... } block
    if let Some(env_node) = doc.get("env") {
        if let Some(children) = env_node.children() {
            for node in children.nodes() {
                let key = node.name().value().to_string();
                if let Some(first) = node.entries().first() {
                    let val = match first.value() {
                        KdlValue::String(s) | KdlValue::RawString(s) => s.clone(),
                        KdlValue::Base10(i) => i.to_string(),
                        KdlValue::Bool(b) => b.to_string(),
                        _ => continue,
                    };
                    config.env.insert(key, val);
                }
            }
        }
    }

    // Parse service "name" { ... } nodes
    let mut parsed_services = Vec::new();
    for node in doc.nodes() {
        if node.name().value() == "service" {
            let name = node
                .entries()
                .first()
                .and_then(|e| match e.value() {
                    KdlValue::String(s) | KdlValue::RawString(s) => Some(s.clone()),
                    _ => None,
                })
                .context("Service node must have a name argument, e.g. service \"pipewire\"")?;

            let mut command = name.clone();
            let mut args = Vec::new();
            let mut restart = "always".to_string();
            let mut wait_for_wayland = true;

            if let Some(children) = node.children() {
                if let Some(KdlValue::String(cmd)) = children.get_arg("command") {
                    command = cmd.clone();
                }
                if let Some(node) = children.get("args") {
                    for entry in node.entries() {
                        if let KdlValue::String(a) = entry.value() {
                            args.push(a.clone());
                        }
                    }
                }
                if let Some(KdlValue::String(rst)) = children.get_arg("restart") {
                    restart = rst.clone();
                }
                if let Some(KdlValue::Bool(wait)) = children.get_arg("wait-for-wayland") {
                    wait_for_wayland = *wait;
                }
            }

            parsed_services.push(SupervisedService {
                name,
                command,
                args,
                restart,
                wait_for_wayland,
            });
        }
    }

    if !parsed_services.is_empty() {
        config.services = parsed_services;
    }

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_default_session_kdl() {
        let doc: KdlDocument = default_session_kdl().parse().unwrap();
        let cfg = parse_session_config(&doc).unwrap();

        assert_eq!(cfg.socket_wait_timeout_secs, 10);
        assert!(cfg.autostart);
        assert_eq!(cfg.compositor, "conchy-wm");
        assert_eq!(cfg.services.len(), 4);
        assert_eq!(cfg.services[0].name, "pipewire");
        assert_eq!(cfg.services[2].name, "conchy-config");
        assert_eq!(cfg.services[2].args, vec!["daemon"]);
    }
}
