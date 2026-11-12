# conchy-config

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Linux%20(Void%20%2F%20Wayland)-green.svg)]()

**`conchy-config`** — единая реактивная подсистема конфигурации, библиотека и CLI для **Conchy OS**.

Обеспечивает целостность окружения благодаря стандартизации всей системы на языке **KDL (KDL Document Language)**, предоставляет фоновый D-Bus сервис `org.conchy.Config` для реактивного применения настроек на лету (аналог `xfconfd` из XFCE) и встроенный Inotify-наблюдатель для поддержки прямого редактирования файлов через Vim/Neovim без потери комментариев.

---

## 🌟 Ключевые возможности

1. **Единый язык KDL для всей системы**:
   - `~/.config/conchy/session.kdl` — параметры сессии, автозапуск, службы супервизора.
   - `~/.config/conchy/wm.kdl` — композитор, раскладка, жесты, отступы, хоткеи.
   - `~/.config/conchy/shell.kdl` — высота и положение панели, виджеты, темы.
2. **D-Bus шина реактивности (`org.conchy.Config`)**:
   - Методы: `Get(channel, key)`, `Set(channel, key, value)`, `List(channel)`.
   - Сигналы: `PropertyChanged(channel, key, value)` для моментального обновления `conchy-shell` и `conchy-wm` без перезапуска.
3. **Inotify File Watcher (Двусторонняя синхронизация с Vim)**:
   - При сохранении файла в текстовом редакторе демон автоматически считывает изменения и рассылает D-Bus сигнал `PropertyChanged`.
   - Умная защита от эхо-зацикливаний (фильтрация собственных программных записей).
4. **Безопасная AST-мутация (Round-trip)**:
   - При изменении настроек через GUI или CLI утилита точечно меняет целевые узлы KDL, **полностью сохраняя комментарии и форматирование пользователя**.
5. **CLI утилита (аналог `xfconf-query`)**:
   - Быстрое чтение, изменение и мониторинг настроек из терминала и скриптов.

---

## 🚀 Использование CLI

### Инициализация стандартных конфигов
```bash
conchy-config init
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
*(В Conchy OS демон автоматически запускается и контролируется супервизором `conchy-session`).*

---

## 💻 Использование как библиотеки в Rust

Для разработчиков компонентов (`conchy-shell`, `conchy-wm`, `conchy-settings`):

```toml
[dependencies]
conchy-config = { git = "https://github.com/conchy-os/conchy-config.git" }
```

### Подписка на изменение настроек на лету:
```rust
use conchy_config::bus::ConfigClient;
use zbus::Connection;
use futures_util::StreamExt;

let conn = Connection::session().await?;
let client = ConfigClient::new(&conn).await?;

let mut stream = client.receive_property_changed().await?;
while let Some(signal) = stream.next().await {
    if let Ok(args) = signal.args() {
        println!("Настройка изменена: [{}] {} = {}", args.channel, args.key, args.value);
        if args.channel == "shell" && args.key == "panel.height" {
            // Перерисовать панель!
        }
    }
}
```

---

## 📄 Лицензия
GPL-3.0-or-later.
