# История изменений

Формат основан на [Keep a Changelog](https://keepachangelog.com/ru/1.1.0/), версия проекта следует [Semantic Versioning](https://semver.org/lang/ru/).

## [0.4.0] — 2026-08-13

### Добавлено

- assertions для HTTP status, заголовков, JSON path, текста и времени ответа;
- последовательный runner всех запросов или выбранной коллекции;
- остановка после первой ошибки;
- отдельный `reqvault-cli` с фильтрами окружения и коллекции;
- компактный JSON-отчёт без тел ответов и секретов;
- exit code `0`, `1` и `2` для использования в CI.

### Изменено

- HTTP 4xx/5xx всегда считается ошибкой collection run, даже если у запроса нет assertions.

## [0.3.0] — 2026-08-13

### Добавлено

- автоматическое обновление OAuth access token по refresh token и повтор запроса после `401`;
- ручное обновление OAuth token из редактора запроса;
- импорт команд cURL с переносом HTTP, multipart, proxy и TLS-настроек;
- локальное разрешение внутренних и внешних OpenAPI `$ref`;
- переносимый `.reqvault.json` bundle для экспорта и импорта workspace;
- Production Guard с обязательным HTTPS, allowlist хостов и блокировкой методов.

### Безопасность

- credential из cURL удаляются до записи запроса и заменяются ссылками на Secret Vault;
- внешние OpenAPI `$ref` читаются только внутри каталога импортируемой спецификации;
- новый UUID назначается при импорте workspace, поэтому системные секреты не связываются автоматически.

## [0.2.0] — 2026-08-13

### Добавлено

- OAuth 2.0 Authorization Code с PKCE и Client Credentials;
- сохранение access/refresh token сразу в системное хранилище;
- HTTP, HTTPS и SOCKS5 proxy;
- custom CA и mTLS с PEM-файлами;
- multipart/form-data с текстовыми полями и файлами;
- импорт Postman Collection 2.x, OpenAPI 3.x и Swagger 2.0;
- локальная история очищенных ответов с явным включением и лимитом;
- скрипт сборки и проверки подписанных Windows-артефактов.

### Безопасность

- credential в полях авторизации и пароль proxy нельзя сохранить открытым текстом;
- значения credential из импортируемых коллекций отбрасываются и заменяются ссылками на Secret Vault;
- OAuth state и PKCE verifier проверяются в Rust.

## [0.1.0] — 2026-08-13

### Добавлено

- desktop-приложение на Tauri 2, Rust, React и TypeScript;
- YAML-workspace с коллекциями запросов и окружениями;
- отправка основных HTTP-методов, query, заголовков и трёх видов тела;
- Bearer, Basic Auth и API key;
- переменные окружения и ссылки на секреты;
- хранение секретов в системном хранилище ОС;
- автоматическая маскировка чувствительных данных;
- безопасный cURL и Security Lens;
- светлая и тёмная тема;
- модульные и интеграционные тесты;
- CI для frontend и Rust.

[0.1.0]: https://github.com/pavel-duke/reqvault/releases/tag/v0.1.0
[0.2.0]: https://github.com/pavel-duke/reqvault/releases/tag/v0.2.0
[0.3.0]: https://github.com/pavel-duke/reqvault/releases/tag/v0.3.0
[0.4.0]: https://github.com/pavel-duke/reqvault/releases/tag/v0.4.0
