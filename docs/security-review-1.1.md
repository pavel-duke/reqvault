# Проверка безопасности 1.1

Проверка охватывает redirect policy, SSRF-защиту, время жизни секретов, mTLS и экспортируемые данные. ReqVault остаётся local-first: в приложении нет аккаунта, телеметрии, аналитики, рекламы или собственного облачного backend.

## Redirect policy

- автоматические редиректы `reqwest` выключены;
- цепочка ограничена десятью переходами;
- origin сравнивается по схеме, hostname и effective port;
- при смене origin удаляются `Authorization`, `Proxy-Authorization`, `Cookie`, API key, OAuth/AWS token и пользовательские заголовки со ссылкой на Secret Vault;
- Production Guard проверяет каждый новый URL и требует явно разрешить неожиданный междоменный переход;
- AWS SigV4 не следует на другой origin, потому что подпись привязана к адресу запроса.

Тесты покрывают HTTP → HTTPS, HTTPS → HTTP, host A → host B и port A → port B. Для смены порта используется реальная пара локальных TCP-серверов, которая проверяет отсутствие credential на втором сервере.

## SSRF и Production Guard

При включённой сетевой политике блокируются:

- `localhost`, loopback и unspecified addresses;
- приватные IPv4, carrier-grade NAT и IPv6 unique local;
- IPv4/IPv6 link-local;
- `169.254.169.254`, ECS и другие известные cloud metadata endpoints;
- hostname, который после DNS-разрешения указывает в один из этих диапазонов.

Guard по умолчанию выключен, поэтому локальная разработка не ломается. В production-режиме локальный API или ожидаемый redirect разрешается явным добавлением hostname в allowlist.

## Секреты и mTLS

- структуры с уже подставленными credential не реализуют `Debug` и не сериализуются;
- чувствительные `HeaderValue` помечены как sensitive;
- временный список известных секретов, OAuth token response, auth/proxy-поля, request body и PEM identity очищаются при освобождении памяти там, где Rust позволяет сделать это надёжно;
- ошибки, история, HAR, safe cURL и frontend получают только очищенные значения;
- незашифрованный приватный PEM-ключ вызывает предупреждение без показа его содержимого; runtime-ошибка не повторяет путь;
- поддержка системного certificate store не добавлена частично: она запланирована вместе с управляемыми сертификатами в 2.3.0.

Полностью исключить копии данных внутри TLS/HTTP-библиотек и защититься от чтения памяти скомпрометированным процессом невозможно. Эта граница явно указана в модели угроз.

## Цепочка поставки

- Dependabot следит за npm, Cargo и GitHub Actions;
- CodeQL анализирует TypeScript, Rust и workflow-файлы;
- gitleaks проверяет всю историю Git;
- npm audit и RustSec блокируют релиз при известных уязвимостях;
- release workflow создаёт SPDX JSON SBOM, SHA-256 и Sigstore provenance attestations.

Локальная проверка перед 1.1: npm — 0 уязвимостей; gitleaks проверил всю историю, утечек нет; RustSec — блокирующих advisory нет. Информационные предупреждения GTK3/glib приходят через Linux-зависимости Tauri и остаются видимыми до обновления upstream-стека.
