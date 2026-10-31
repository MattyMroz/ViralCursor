# ViralCursor

Aplikacja Tauri 2 tylko na Windows: zamienia systemowy kursor na animowaną dłoń z dema pokey, na każdym monitorze.

## Komendy

```bash
bun install
bun run dev      # uruchomienie bez pakowania
bun run build    # src-tauri/target/release/ViralCursor.exe
```

Wymaga Rusta i Buna. Repo nie ma testów.

## Mapa

- `src/`: front. `overlay.js` (eskalacja pokes, tryb karabinu maszynowego), `panel.js` i `index.html` (panel sterowania), `overlay.css`.
- `src-tauri/src/`: backend Win32. `cursor.rs` (ukrycie kursora przez `SetSystemCursor`), `hook.rs` (globalne hooki myszy i klawiatury, awaryjny skrót Ctrl+Alt+Q), `fullscreen.rs`, `lib.rs` (okno overlay i spięcie całości).
- `reference/pokey_cursor_fast_hold_fx_shake.html`: demo, źródło grafiki SVG.
- `tools/`: `build-overlay.py` generuje `src/overlay.html`, `build-icon.mjs` generuje `icon.png`.

## Pułapki

- `src/overlay.html` jest generowany: nie edytuj go ręcznie, zmieniaj demo w `reference/` i uruchom `python tools/build-overlay.py`.
- Overlay jest przezroczysty i przepuszcza kliknięcia, więc pozycja kursora i Ctrl przychodzą z hooków w Ruście jako zdarzenia Tauri, nie z DOM.
- Ukrycie kursora to zmiana systemowa. Ctrl+Alt+Q musi dalej przywracać zwykły kursor, także gdy okno jest schowane.
- Ikonę odtwarzasz tak: `bun tools/build-icon.mjs && bunx tauri icon icon.png`.

## Git

- Commit: nagłówek Conventional Commits po angielsku, `type(scope): opis`, ze scope'em z listy w `.pre-commit-config.yaml`; po pustej linii streszczenie „co i dlaczego”; potem sekcja `Validation:` z kontrolami uruchomionymi na finalnym indeksie. Format: https://github.com/MattyMroz/agents/blob/main/presets/hooks/references/commit-message.md
- Kontrole uruchamiaj przez `mm-validate run "<kontrola>" -- <komenda>`, blok `Validation:` bierz z `mm-validate list`. Hooki `conventional-pre-commit` i `commit-validation` odrzucą inny commit.
- Pliki dodawaj po nazwie, nigdy `git add .`; bez `--no-verify`. Worktree'y tylko w `.worktrees/<nazwa>`.
