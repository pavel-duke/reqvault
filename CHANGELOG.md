# История изменений

Формат основан на [Keep a Changelog](https://keepachangelog.com/ru/1.1.0/), версия проекта следует [Semantic Versioning](https://semver.org/lang/ru/).

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
