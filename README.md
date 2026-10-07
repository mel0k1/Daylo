# Daylo

Лаунчер Minecraft для Windows, macOS и Linux: ядро на Rust, интерфейс на React.
Игра, библиотеки и Java скачиваются сами, аккаунт не обязателен.

Готовые сборки и моды ставятся из каталогов Modrinth, CurseForge и FTB,
загрузчики Fabric, Quilt, Forge и NeoForge поддерживаются.

## Стек

- Ядро — Rust + Tauri 2
- Интерфейс — React 19, TypeScript, Vite
- Состояние — zustand
- Сборка — bun

## Разработка

Нужен bun и Rust-тулчейн (по [rustup.rs](https://rustup.rs)).

```bash
bun install
bun run tauri dev     # запуск с живой перезагрузкой
bun run tauri build   # сборка установщика
bun run typecheck     # проверка типов
```

## Документы

- [DESIGN.md](DESIGN.md) — заметки о внешнем виде
- [LICENSE](LICENSE) — GPL-3.0-only

Daylo не связан с Mojang AB и Microsoft. Minecraft — товарный знак Mojang AB.
