# ReqVault

ReqVault — локальный desktop-клиент для REST API. Запросы и окружения хранятся как понятные YAML-файлы, а токены и пароли — отдельно в защищённом хранилище операционной системы.

Проект не требует аккаунта, облачного сервиса или собственного backend. Workspace можно положить в Git и использовать в команде, не добавляя секреты в репозиторий.

> Версия `0.1.0` — рабочий MVP. Формат файлов имеет номер версии, но до `1.0` обратная совместимость ещё не гарантируется.

## Возможности

- workspace в обычной папке: `reqvault.yaml`, `requests/**/*.yaml`, `environments/*.yaml`;
- методы GET, POST, PUT, PATCH, DELETE, HEAD и OPTIONS;
- query-параметры, заголовки, JSON, raw text и `application/x-www-form-urlencoded`;
- Bearer Token, Basic Auth и API key в заголовке или query;
- переменные окружения вида `{{BASE_URL}}`;
- ссылки на секреты вида `{{secret:API_TOKEN}}`;
- системное хранилище секретов: Windows Credential Manager, macOS Keychain и Secret Service в Linux;
- таймаут и управление редиректами;
- просмотр статуса, времени, размера, заголовков и тела ответа;
- форматирование JSON и отдельный raw-режим;
- безопасный cURL без значений секретов;
- Security Lens: HTTPS, хост, число ссылок на секреты и предупреждение о секрете в URL;
- светлая и тёмная тема;
- отправка запроса по `Ctrl+Enter` или `Cmd+Enter`.

## Как устроено хранение

Пример workspace есть в каталоге [`examples/sample-workspace`](examples/sample-workspace).

```text
my-api/
├── reqvault.yaml
├── environments/
│   └── local.yaml
└── requests/
    └── users/
        └── get-user.yaml
```

`reqvault.yaml` содержит только версию формата, UUID и имя workspace:

```yaml
format_version: 1
id: 7f39e4a8-eeb8-4f2b-a866-226bf1a325d8
name: My API
```

Окружение содержит обычные, несекретные значения:

```yaml
format_version: 1
name: local
variables:
  BASE_URL: https://api.example.test
  USER_ID: "42"
```

В запросе можно использовать оба типа ссылок:

```yaml
url: "{{BASE_URL}}/users/{{USER_ID}}"
auth:
  type: bearer
  token: "{{secret:API_TOKEN}}"
```

Значение `API_TOKEN` добавляется через окно «Секреты». В YAML записывается только имя ссылки. ReqVault получает значение из системного хранилища непосредственно перед отправкой запроса и маскирует известные секреты в ответах и ошибках.

Секреты разделены по UUID workspace. Одинаковое имя секрета в двух workspace относится к разным записям. Подробнее: [`docs/threat-model.md`](docs/threat-model.md).

## Быстрый старт для разработки

Нужно установить:

- Node.js 22.19 или новее;
- Rust stable;
- системные зависимости Tauri 2 для своей ОС.

Для Ubuntu/Debian обычно нужны:

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

Установка зависимостей и запуск desktop-приложения:

```bash
npm ci
npm run tauri dev
```

Только frontend в браузере запускается командой `npm run dev`, но системные функции Tauri там недоступны.

## Проверки

Frontend:

```bash
npm run lint
npm run typecheck
npm test
npm run build
```

Rust:

```bash
cd src-tauri
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo check
```

Production-сборка и установщики:

```bash
npm run tauri build
```

Готовые файлы появятся в `src-tauri/target/release/bundle`. Сборка выполняется для текущей операционной системы; для других ОС нужно запускать её на соответствующей платформе.

## Ограничения версии 0.1.0

- нет импорта из Postman/OpenAPI и экспорта workspace;
- нет OAuth 2.0;
- нет proxy, custom CA и mTLS;
- нет multipart/form-data и загрузки файлов;
- нет истории ответов и фонового сохранения ответа на диск: это сделано намеренно, чтобы незаметно не собирать архив production payloads;
- бинарные ответы показываются как текст с заменой некорректных UTF-8 символов;
- запросы выполняются последовательно из одного окна;
- изменение YAML во внешнем редакторе требует повторного открытия workspace.

## Что дальше

### 0.2

- Production Guard;
- правила безопасности workspace;
- custom CA;
- proxy;
- mTLS.

### 0.3

- OAuth 2.0 / PKCE;
- импорт cURL;
- импорт OpenAPI;
- импорт Postman.

### 0.4

- CLI;
- запуск API-тестов из терминала;
- CI runner.

### Потом

- GraphQL;
- gRPC;
- WebSocket;
- SSE.

Это планы без обещанных сроков. Незаконченные функции не включаются в релизы только ради roadmap.

## Безопасность и участие

- Об уязвимостях сообщайте по инструкции в [`SECURITY.md`](SECURITY.md).
- Правила разработки и отправки изменений находятся в [`CONTRIBUTING.md`](CONTRIBUTING.md).
- Архитектура описана в [`docs/architecture.md`](docs/architecture.md).

ReqVault распространяется по лицензии [Apache License 2.0](LICENSE).
