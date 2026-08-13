use std::{path::PathBuf, process::ExitCode};

use crate::{
    models::{CollectionRunOptions, CollectionRunReport},
    runner,
};

pub async fn run(arguments: impl IntoIterator<Item = String>) -> ExitCode {
    match parse(arguments) {
        Ok(Command::Help) => {
            print_help();
            ExitCode::SUCCESS
        }
        Ok(Command::Version) => {
            println!("reqvault-cli {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(Command::Run {
            workspace,
            options,
            report,
        }) => match runner::run_workspace(&workspace, &options).await {
            Ok(result) => finish_run(&result, report.as_ref()),
            Err(error) => {
                eprintln!("Ошибка: {error}");
                ExitCode::from(2)
            }
        },
        Err(error) => {
            eprintln!("Ошибка: {error}\n");
            print_help();
            ExitCode::from(2)
        }
    }
}

enum Command {
    Help,
    Version,
    Run {
        workspace: PathBuf,
        options: CollectionRunOptions,
        report: Option<PathBuf>,
    },
}

fn parse(arguments: impl IntoIterator<Item = String>) -> Result<Command, String> {
    let mut arguments = arguments.into_iter();
    let _executable = arguments.next();
    let Some(command) = arguments.next() else {
        return Ok(Command::Help);
    };
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        return Ok(Command::Help);
    }
    if matches!(command.as_str(), "--version" | "-V") {
        return Ok(Command::Version);
    }
    if command != "run" {
        return Err(format!("Неизвестная команда {command}"));
    }
    let workspace = arguments
        .next()
        .filter(|value| !value.starts_with('-'))
        .map(PathBuf::from)
        .ok_or_else(|| "После run укажи путь к workspace".to_string())?;
    let mut options = CollectionRunOptions::default();
    let mut report = None;
    let rest = arguments.collect::<Vec<_>>();
    let mut index = 0;
    while index < rest.len() {
        match rest[index].as_str() {
            "--environment" | "-e" => {
                index += 1;
                options.environment = Some(
                    rest.get(index)
                        .cloned()
                        .ok_or_else(|| "После --environment укажи имя".to_string())?,
                );
            }
            "--collection" | "-c" => {
                index += 1;
                options.collection = Some(
                    rest.get(index)
                        .cloned()
                        .ok_or_else(|| "После --collection укажи имя".to_string())?,
                );
            }
            "--report" | "-r" => {
                index += 1;
                report =
                    Some(PathBuf::from(rest.get(index).ok_or_else(|| {
                        "После --report укажи путь к JSON".to_string()
                    })?));
            }
            "--stop-on-failure" => options.stop_on_failure = true,
            value => return Err(format!("Неизвестный параметр {value}")),
        }
        index += 1;
    }
    Ok(Command::Run {
        workspace,
        options,
        report,
    })
}

fn finish_run(report: &CollectionRunReport, report_path: Option<&PathBuf>) -> ExitCode {
    for result in &report.results {
        let mark = if result.passed { "PASS" } else { "FAIL" };
        let status = result
            .status
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string());
        let duration = result
            .duration_ms
            .map(|value| format!("{value} ms"))
            .unwrap_or_else(|| "-".to_string());
        println!(
            "{mark:4} {:7} {:3} {:>8}  {}",
            result.method, status, duration, result.request_name
        );
        if let Some(error) = &result.error {
            println!("     {error}");
        }
        for assertion in result.assertions.iter().filter(|item| !item.passed) {
            println!(
                "     {}: ожидалось {}, получено {}",
                assertion.label, assertion.expected, assertion.actual
            );
        }
    }
    println!(
        "\nИтого: {}, успешно: {}, ошибок: {}, время: {} ms",
        report.total, report.passed, report.failed, report.duration_ms
    );

    if let Some(path) = report_path {
        let json = match serde_json::to_string_pretty(report) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("Не удалось подготовить JSON-отчёт: {error}");
                return ExitCode::from(2);
            }
        };
        if let Err(error) = std::fs::write(path, json) {
            eprintln!("Не удалось записать {}: {error}", path.display());
            return ExitCode::from(2);
        }
        println!("Отчёт: {}", path.display());
    }

    if report.failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn print_help() {
    println!(
        "ReqVault CLI\n\n\
Использование:\n  reqvault-cli run <workspace> [параметры]\n\n\
Параметры:\n  -e, --environment <имя>   Окружение\n  -c, --collection <имя>    Только одна коллекция\n  -r, --report <файл.json>  Записать JSON-отчёт\n      --stop-on-failure     Остановиться после первой ошибки\n  -h, --help                Показать справку\n  -V, --version             Показать версию\n\n\
Exit code: 0 — все проверки прошли, 1 — есть упавшие проверки, 2 — ошибка запуска."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_run_options() {
        let command = parse([
            "reqvault-cli".to_string(),
            "run".to_string(),
            "./workspace".to_string(),
            "--environment".to_string(),
            "testing".to_string(),
            "--collection".to_string(),
            "users".to_string(),
            "--stop-on-failure".to_string(),
        ])
        .unwrap();
        let Command::Run { options, .. } = command else {
            panic!("expected run command");
        };
        assert_eq!(options.environment.as_deref(), Some("testing"));
        assert_eq!(options.collection.as_deref(), Some("users"));
        assert!(options.stop_on_failure);
    }
}
