# conchy-config

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Linux%20(Void%20%2F%20Arch%20%2F%20Wayland)-green.svg)]()

**`conchy-config`** — единая реактивная подсистема конфигурации, библиотека и CLI для **Conchy OS** (модель rolling release: без фиксированных релизных версий, пакеты собираются из `main` по дате).

Обеспечивает стандартизацию конфигурации всей системы на языке **KDL (KDL Document Language)**, предоставляет фоновый D-Bus сервис `org.conchy.Config` (`/org/conchy/Config`) для реактивного применения настроек на лету (аналог `xfconfd` из XFCE) и встроенный Inotify-наблюдатель (`notify`) для поддержки прямого редактирования файлов в Vim/Neovim без потери комментариев и форматирования.

---

## 🌟 Ключевые возможности

1. **Единый язык KDL и схемы каналов (`src/schema/`)**:
   По умолчанию файлы располагаются в `$XDG_CONFIG_HOME/conchy/` (иначе `~/.config/conchy/`, иначе `/etc/conchy/`; путь можно переопределить ключом `-C` / `--config-dir`).

   При полной инициализации (`conchy-config init`, `conchy-config daemon`, `ConfigStore::new`) создаются отсутствующие файлы четырёх стандартных каналов:
   - `session.kdl` (`src/schema/session.rs` — `SessionConfig`, `SupervisedService`, `parse_session_config`) — параметры Wayland-сессии (`session { socket-wait-timeout, autostart }`), переменные окружения (`env`), композитор (`compositor`) и службы супервизора (`service "..." { command, args, restart, wait-for-wayland }`).
   - `wm.kdl` (`src/schema/wm.rs` — `default_wm_kdl`) — настройки композитора `conchy-wm`: мониторы (`output`), раскладка и рамки (`layout`, `gaps`, `focus-ring`, `border`), анимации (`animations`) и горячие клавиши (`binds`).
   - `shell.kdl` (`src/schema/shell.rs` — `default_shell_kdl`) — настройки оболочки `conchy-shell`: панель (`bar`), док (`dock`), лаунчер (`launcher`) и окно настроек (`settings`). Стандартный шаблон задаёт `bar { position "top"; style "floating"; size 32.0 }`, а остальные блоки используют значения по умолчанию. **Цвета и темы не хранятся в `shell.kdl`** — они живут отдельно в `theme.kdl` (+ необязательном `appearance.kdl`).
   - `files.kdl` (`src/schema/files.rs` — `FilesConfig`, `SortBy`, `FolderSize`, `parse_files_config`) — настройки файлового менеджера `conchy-files`: список (`view`: `show-hidden`, `folders-first`, `sort-by`, `sort-order`, `folder-size`), окно (`window`: `width`, `height`, `sidebar-width`), терминал (`terminal.command`), тема иконок и шрифт (`theme`: `icon-theme`, `font-size`), нижняя панель анализа места и свойств (`panel`: `visible`, `height`) и дисковый кэш (`cache`: `enabled`, `max-size-mb`, `max-age-days`).

   Дополнительно модуль `src/schema/appearance.rs` реализует схему **`appearance`** для портала `xdg-desktop-portal-conchy`:
   - `appearance_from_theme_kdl(theme, overrides)` разбирает `theme.kdl` (определяя `ColorScheme::Dark` / `ColorScheme::Light` по HSL-светлоте `base16.base00` и акцент из `colors.accent` с откатом к `base16.base0d`; поддерживаются hex, `rgb()`, `hsl()`, `oklch()`) и необязательный `appearance.kdl` (`appearance { color-scheme, accent, contrast, reduced-motion }`).
   - Переопределения вынесены в отдельный `appearance.kdl`, так как `conchy-shell` при сохранении темы переписывает `theme.kdl` целиком.

2. **D-Bus шина реактивности (`org.conchy.Config`)**:
   - Объект: `/org/conchy/Config`.
   - Методы: `Get(channel, key)`, `Set(channel, key, value)`, `List(channel)`.
   - Сигнал: `PropertyChanged(channel, key, value)` для моментального обновления `conchy-shell`, `conchy-wm`, `conchy-files` и других компонентов без перезапуска.

3. **Inotify File Watcher (двусторонняя синхронизация с редактором)**:
   - При изменении `*.kdl` в каталоге конфигурации (временные файлы с ведущей точкой вроде `.wm.tmp` игнорируются) демон сравнивает актуальные свойства со снимком в памяти и рассылает сигнал `PropertyChanged` для изменившихся ключей.
   - Защита от эхо-зацикливаний (`ConfigStore::is_recent_programmatic_write`): недавние программные записи через `ConfigStore` (в пределах 400 мс) пропускаются.

4. **Безопасная AST-мутация (Round-trip) и атомарная запись**:
   - При изменении настроек через GUI, D-Bus или CLI точечно обновляются целевые узлы KDL (`ast::{get_value, set_value, list_values}`), сохраняя комментарии и форматирование пользователя.
   - `ConfigStore::save_document` записывает документ во временный файл `.<channel>.tmp` в том же каталоге и выполняет атомарный `rename`.

5. **CLI утилита (аналог `xfconf-query`)**:
   - Быстрое чтение, изменение, просмотр списка параметров и мониторинг событий из терминала и скриптов.

---

## 🚀 Использование CLI

### Глобальные параметры
- `-C, --config-dir <DIR>` — пользовательский каталог конфигурации вместо `$XDG_CONFIG_HOME/conchy` / `~/.config/conchy` / `/etc/conchy`.
- `-l, --log-level <LOG_LEVEL>` — уровень логирования (`trace`, `debug`, `info`, `warn`, `error`; также переменная `CONCHY_LOG`, по умолчанию `info`).

> **Примечание про `-C` и D-Bus:** команды `get` и `set` сначала пытаются обратиться к активному D-Bus сервису `org.conchy.Config` на сессионной шине и переходят к прямому чтению/записи в каталог из `--config-dir` только если сервис недоступен. Команды `init`, `list` и `daemon` работают непосредственно с указанным `--config-dir`.

### Инициализация стандартных конфигов
```bash
conchy-config init
# Или в отдельном каталоге:
conchy-config -C /tmp/conchy-test init
```

### Чтение параметров
```bash
conchy-config get wm layout.gaps
# 16

conchy-config get shell bar.size
# 32
```

### Изменение параметров (с мгновенным уведомлением через D-Bus)
```bash
conchy-config set wm layout.gaps 24
conchy-config set shell bar.position "bottom"
```

### Просмотр всех параметров канала
```bash
conchy-config list wm
conchy-config list session
conchy-config list files
```

### Мониторинг изменений в реальном времени
```bash
conchy-config watch
# Или с фильтром по каналу:
conchy-config watch wm
```

### Запуск D-Bus демона и Inotify-наблюдателя
```bash
conchy-config daemon
```
*(В Conchy OS демон автоматически запускается и контролируется супервизором `conchy-session`.)*

---

## 💻 Использование как библиотеки в Rust

Для разработчиков компонентов (`conchy-shell`, `conchy-wm`, `conchy-files`, `conchy-session`, `xdg-desktop-portal-conchy`):

```toml
[dependencies]
conchy-config = { git = "https://github.com/conchy-os/conchy-config.git" }
```

### `ConfigStore::open` + `ensure_channel` vs `ConfigStore::new` + `ensure_defaults`

- **`ConfigStore::new(custom_dir)`** (вызывает **`ensure_defaults()`**) — создаёт каталог конфигурации (если его нет) и записывает файлы по умолчанию для **всех** каналов (`session.kdl`, `wm.kdl`, `shell.kdl`, `files.kdl`). Подходит для `conchy-config init` и `conchy-config daemon`.
- **`ConfigStore::open(custom_dir)`** — открывает путь к каталогу конфигурации, **ничего не создавая на диске**. В паре с **`ensure_channel(channel, content)`** используется отдельными приложениями (`conchy-files`, `conchy-shell` и т.д.), чтобы инициализировать только свой канал (если файл ещё отсутствует) и не создавать чужие конфиги:

```rust
use anyhow::Result;
use conchy_config::schema::default_files_kdl;
use conchy_config::{parse_files_config, ConfigStore};

fn load_files_settings() -> Result<()> {
    let store = ConfigStore::open(None);
    store.ensure_channel("files", default_files_kdl())?;

    let doc = store.load_document("files")?;
    let cfg = parse_files_config(&doc)?;
    println!("show_hidden = {}", cfg.show_hidden);
    Ok(())
}
```

### Разбор темы и внешнего вида (`appearance`)

```rust
use conchy_config::{appearance_from_theme_kdl, ColorScheme};

let appearance = appearance_from_theme_kdl(&theme_kdl, appearance_kdl.as_deref())?;
let portal_scheme = appearance.color_scheme.portal_value(); // 0 | 1 | 2
assert!(matches!(
    appearance.color_scheme,
    ColorScheme::NoPreference | ColorScheme::Dark | ColorScheme::Light
));
```

### Подписка на изменение настроек на лету

```rust
use conchy_config::bus::ConfigClient;
use futures_util::StreamExt;
use zbus::Connection;

let conn = Connection::session().await?;
let client = ConfigClient::new(&conn).await?;

let mut stream = client.receive_property_changed().await?;
while let Some(signal) = stream.next().await {
    if let Ok(args) = signal.args() {
        println!(
            "Настройка изменена: [{}] {} = {}",
            args.channel, args.key, args.value
        );
        if args.channel == "shell" && args.key == "bar.size" {
            // Перерисовать панель!
        }
    }
}
```

---

## 📄 Лицензия
GPL-3.0-or-later.
