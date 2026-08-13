# ReqVault CLI и CI

`reqvault-cli` запускает те же запросы и проверки, что и desktop-приложение. Это позволяет сначала отладить коллекцию локально, затем без дублирования включить её в pipeline.

## Команды

```bash
reqvault-cli run ./my-workspace
reqvault-cli run ./my-workspace --environment staging
reqvault-cli run ./my-workspace --collection users --report artifacts/reqvault.json
```

Коды завершения:

- `0` — запросы и assertions прошли;
- `1` — сервер ответил 4xx/5xx или проверка не прошла;
- `2` — workspace, окружение или параметры запуска некорректны.

JSON-отчёт не содержит тела ответа и значений из Secret Vault. Перед запуском в CI передайте нужные значения через системное хранилище runner или используйте отдельное тестовое окружение без постоянных production-ключей.

## GitHub Actions

```yaml
name: API checks
on: [push, pull_request]

jobs:
  api:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run ReqVault collection
        run: reqvault-cli run ./api --environment ci --report reqvault-report.json
      - uses: actions/upload-artifact@v4
        if: always()
        with:
          name: reqvault-report
          path: reqvault-report.json
```

Не печатайте токены в аргументах команды и не добавляйте их в YAML. Для production endpoint включите Production Guard и ограничьте allowlist хостов.
