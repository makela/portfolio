// Light/dark theme toggle, shared by every page.
//
// Loaded as a blocking script in <head>, so a saved choice is applied before
// the first paint and the page never flashes the wrong theme. Without a saved
// choice the page follows the system setting (prefers-color-scheme). Choosing
// the theme the system already uses clears the saved choice, so the page goes
// back to following the system.
(() => {
  const KEY = "theme";
  const root = document.documentElement;
  const systemDark = matchMedia("(prefers-color-scheme: dark)");

  // localStorage can throw (blocked storage, some private modes).
  const load = () => {
    try {
      return localStorage.getItem(KEY);
    } catch {
      return null;
    }
  };
  const save = (theme) => {
    try {
      if (theme) localStorage.setItem(KEY, theme);
      else localStorage.removeItem(KEY);
    } catch {}
  };

  const saved = load();
  if (saved === "light" || saved === "dark") root.dataset.theme = saved;

  const system = () => (systemDark.matches ? "dark" : "light");
  const current = () => root.dataset.theme || system();

  // Update the button label and the browser UI color to match the theme.
  function sync() {
    const theme = current();
    const button = document.getElementById("theme-toggle");
    if (button) {
      button.dataset.current = theme;
      button.setAttribute(
        "aria-label",
        theme === "dark" ? "Switch to light theme" : "Switch to dark theme",
      );
    }
    const bg = getComputedStyle(root).getPropertyValue("--bg").trim();
    for (const meta of document.querySelectorAll('meta[name="theme-color"]')) {
      meta.content = bg;
    }
  }

  function toggle() {
    const next = current() === "dark" ? "light" : "dark";
    if (next === system()) {
      delete root.dataset.theme;
      save(null);
    } else {
      root.dataset.theme = next;
      save(next);
    }
    sync();
  }

  systemDark.addEventListener("change", sync);
  document.addEventListener("DOMContentLoaded", () => {
    const button = document.getElementById("theme-toggle");
    if (!button) return;
    button.addEventListener("click", toggle);
    button.hidden = false; // hidden without JavaScript, where it can't work
    sync();
  });
})();
