import { useEffect, useState } from "react";
import "./App.css";

type Theme = "light" | "dark";

function App() {
  const [theme, setTheme] = useState<Theme>(() => {
    const savedTheme = window.localStorage.getItem("reqvault.theme");
    if (savedTheme === "light" || savedTheme === "dark") return savedTheme;
    return window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light";
  });

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    window.localStorage.setItem("reqvault.theme", theme);
  }, [theme]);

  return (
    <main className="app-shell">
      <header className="topbar">
        <div className="brand" aria-label="ReqVault">
          <span className="brand-mark" aria-hidden="true">RV</span>
          <span>ReqVault</span>
        </div>
        <button
          className="icon-button"
          type="button"
          onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
          aria-label={theme === "dark" ? "Включить светлую тему" : "Включить тёмную тему"}
          title={theme === "dark" ? "Светлая тема" : "Тёмная тема"}
        >
          {theme === "dark" ? "☀" : "☾"}
        </button>
      </header>

      <section className="start-screen">
        <div className="start-card">
          <span className="start-mark" aria-hidden="true">RV</span>
          <h1>Открой workspace</h1>
          <p>
            Запросы хранятся в обычных YAML-файлах. Токены и пароли остаются
            в системном хранилище ОС.
          </p>
          <div className="start-actions">
            <button className="primary-button" type="button">Создать workspace</button>
            <button className="secondary-button" type="button">Открыть папку</button>
          </div>
        </div>
      </section>
    </main>
  );
}

export default App;
