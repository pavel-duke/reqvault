# Цепочка поставки

ReqVault собирается только из зафиксированных `package-lock.json` и `Cargo.lock`. Проверки в GitHub Actions не используют `continue-on-error` для уязвимостей, утечек секретов, статического анализа и отсутствующих артефактов.

## Автоматические проверки

- `frontend.yml`: lint, типы, тесты, production build и `npm audit` для high/critical уязвимостей;
- `rust.yml`: форматирование, Clippy, тесты и сборка на Windows, macOS и Linux;
- `security.yml`: RustSec, gitleaks по истории Git и SPDX JSON SBOM;
- `codeql.yml`: CodeQL для JavaScript/TypeScript, Rust и GitHub Actions;
- Dependabot еженедельно проверяет npm, Cargo и версии GitHub Actions.

Информационные RustSec advisory не скрываются. На момент проверки 1.1 они относятся к GTK3-ветке Tauri для Linux, включая `glib 0.18`; известных уязвимостей npm и RustSec с уровнем, блокирующим релиз, нет. Обновления Tauri и его Linux webview-стека отслеживаются через Dependabot и проверяются до каждого релиза.

## Релизные артефакты

Workflow `bundle.yml` для каждого desktop-артефакта:

1. собирает установщик на нативном GitHub-hosted runner;
2. формирует `SHA256SUMS-<platform>.txt`;
3. создаёт подписанную Sigstore provenance attestation через GitHub;
4. публикует установщики, хэши и отдельный SPDX JSON SBOM как workflow artifacts.

Проверка attestations после публикации:

```bash
gh attestation verify ReqVault_1.1.0_x64-setup.exe --repo pavel-duke/reqvault
```

Проверка SHA-256 в PowerShell:

```powershell
Get-FileHash .\ReqVault_1.1.0_x64-setup.exe -Algorithm SHA256
```

Attestation подтверждает происхождение сборки, но не заменяет подпись Windows Authenticode. Установщики пока не подписаны коммерческим сертификатом, поэтому SmartScreen может показать предупреждение.
