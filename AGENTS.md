# AGENTS.md — `conchy-config`

Guidance for AI coding agents and contributors working in `/home/michael/Projects/conchy/conchy-config`.

---

## 1. Shared Conchy OS Rules

- **Rolling release**: Conchy OS has no semantic version releases or versioned changelogs; packaging is date-based from `main`. Do not introduce version-tag release workflows or contradict rolling-release assumptions in docs or packaging.
- **Rust-only for first-party code**: All Conchy OS components are implemented in pure Rust with no GTK or Qt dependencies.
- **Unix philosophy**: One utility = one task. `conchy-config` is strictly responsible for reactive KDL configuration storage/AST mutation, the `org.conchy.Config` D-Bus service, Inotify watching, CLI queries/updates, and shared config schema helpers.
- **Target distributions**: **Void Linux** (`glibc` / `runit` / `xbps`) is the primary target; **Arch Linux** is secondary.
- **Git branching**: Single long-lived branch `main`. Work on short-lived PR branches and delete them after merge. Never force-push `main`.
- **Roadmap workflow**: When working on roadmap tasks, check `conchy-os/ROADMAP.md` and `conchy-os/roadmap.json` for `depends` and architectural `decisions`, verify all `done_when` criteria before marking work complete, and update task status in the same PR.
- **Zero-warning Rust verification gate**: Every change must pass with zero warnings:
  ```bash
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace
  ```

---

## 2. Safety & Session Isolation Rules

- **Never touch the live user desktop session**:
  - Do NOT run `hyprctl reload` or any compositor IPC against a live session.
  - Do NOT kill, signal, or restart running user session processes.
  - Do NOT edit files under `~/.config`, `~/.local`, or sibling repositories when working in `conchy-config`.
- **NEVER run `conchy-config daemon` on the live session D-Bus** while `org.conchy.Config` is active (or against the host `DBUS_SESSION_BUS_ADDRESS`). Doing so contests or interacts with the live desktop configuration bus.
- **Isolate config directories and D-Bus when testing**:
  - Always use `-C "$(mktemp -d)"` / `--config-dir` and `XDG_CONFIG_HOME="$(mktemp -d)"` for manual CLI checks or tests touching config paths.
  - **Important `get`/`set` D-Bus precedence (`src/main.rs`)**: `conchy-config get` and `conchy-config set` attempt to connect to `org.conchy.Config` on the session D-Bus *before* falling back to `--config-dir`. On a host with a live session bus, always unset `DBUS_SESSION_BUS_ADDRESS` (e.g. `env -u DBUS_SESSION_BUS_ADDRESS`) when running CLI tests or `cargo test` so commands cannot accidentally read or mutate `~/.config/conchy/*.kdl` via the live daemon.
  - To test `daemon` + `watch` manually, launch a private bus via `dbus-run-session` combined with `-C "$(mktemp -d)"`.
- **The `dconf` / `gsettings` trap**: Never write to host `dconf` or `gsettings` during tests or portal/appearance checks.
- **Process cleanup safety**: Never use `pkill -f` with a pattern that could match your own shell or command invocation.
- **Shared libraries**: Never load unverified `.so` files.

---

## 3. Repository Specifics (`conchy-config`)

### Dependency Role
- `conchy-config` has **no sibling path dependencies** in `Cargo.toml`.
- Downstream Conchy OS repositories — including `conchy-session`, `conchy-shell`, `conchy-files`, and `conchy-portal` (`xdg-desktop-portal-conchy`) — depend on `conchy-config`. Keep public APIs (`src/lib.rs`, `src/store.rs`, `src/bus.rs`, `src/schema/*`) backwards-compatible or coordinate breaking changes explicitly across dependents.

### Module Overview
- `src/lib.rs` — crate root and public re-exports.
- `src/ast.rs` — comment-preserving KDL AST dot-path operations (`get_value`, `set_value`, `list_values`).
- `src/store.rs` — `ConfigStore`:
  - `ConfigStore::new` + `ensure_defaults`: creates the config dir and writes missing defaults for all four standard channels (`session`, `wm`, `shell`, `files`).
  - `ConfigStore::open` + `ensure_channel`: opens the config dir without creating anything, allowing an app to initialize only its own channel.
  - Atomic saves via `.<channel>.tmp` + `rename`, plus programmatic-write tracking (`is_recent_programmatic_write`, 400 ms window).
- `src/bus.rs` — `org.conchy.Config` (`/org/conchy/Config`) D-Bus `ConfigServer`, `ConfigProxy`, `ConfigClient`, and `PropertyChanged` signal.
- `src/watcher.rs` — `FileWatcher` (`notify` / Inotify) broadcasting `PropertyChanged` on manual `*.kdl` edits while ignoring dot/temp files and recent programmatic writes.
- `src/daemon.rs` — `Daemon::run` binding `org.conchy.Config` and starting `FileWatcher`.
- `src/cli.rs`, `src/main.rs` — `conchy-config` CLI (`-C`/`--config-dir`, `-l`/`--log-level`, `init`, `get`, `set`, `list`, `watch`, `daemon`).
- `src/schema/`:
  - `session.rs` — `session.kdl` (`SessionConfig`, `SupervisedService`, `parse_session_config`, `default_session_kdl`).
  - `wm.rs` — `wm.kdl` (`default_wm_kdl`).
  - `shell.rs` — `shell.kdl` (`default_shell_kdl`; configures bar/dock/launcher/settings, whereas colors/themes live in `theme.kdl` + optional `appearance.kdl`).
  - `files.rs` — `files.kdl` (`FilesConfig`, `SortBy`, `FolderSize`, `parse_files_config`, `default_files_kdl`).
  - `appearance.rs` — `appearance_from_theme_kdl`, `Appearance`, `ColorScheme`, `parse_color` (reads `theme.kdl` + optional `appearance.kdl` for `xdg-desktop-portal-conchy`).
- `tests/integration.rs` — CLI integration tests (`init`, `get`, `set`, `list`, comment preservation).

### Comment & Docstring Style
- Core modules (`ast.rs`, `bus.rs`, `cli.rs`, `daemon.rs`, `watcher.rs`, core `store.rs`, `schema/session.rs`, `schema/wm.rs`, `schema/shell.rs`) use **English** comments/docstrings.
- Newer schema/files and appearance modules (`src/schema/files.rs`, `src/schema/appearance.rs`, and `ConfigStore::open` / `ensure_channel` in `src/store.rs`) use **Russian** comments/docstrings.
- Preserve existing comments unrelated to your edits and match the language of the file/section you are modifying.

---

## 4. Build, Check & Safe Testing Commands

```bash
# Build
cargo build --workspace

# Format (apply / check)
cargo fmt --all
cargo fmt --all -- --check

# Clippy (strict, all targets)
cargo clippy --workspace --all-targets -- -D warnings

# Run all unit and integration tests (isolated from live D-Bus and ~/.config)
env -u DBUS_SESSION_BUS_ADDRESS XDG_CONFIG_HOME="$(mktemp -d)" cargo test --workspace
```

Safe manual CLI testing with an isolated temporary config directory:

```bash
TMP_CFG="$(mktemp -d)"
env -u DBUS_SESSION_BUS_ADDRESS cargo run -- -C "$TMP_CFG" init
env -u DBUS_SESSION_BUS_ADDRESS cargo run -- -C "$TMP_CFG" get wm layout.gaps
env -u DBUS_SESSION_BUS_ADDRESS cargo run -- -C "$TMP_CFG" set wm layout.gaps 24
env -u DBUS_SESSION_BUS_ADDRESS cargo run -- -C "$TMP_CFG" list wm
rm -rf "$TMP_CFG"
```

Void CI runs in `.github/workflows/ci.yml` inside `ghcr.io/void-linux/void-glibc-full:latest` and enforces `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`.
