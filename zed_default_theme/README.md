# Zed default theme — extracted source

Theme code extracted from [zed-industries/zed](https://github.com/zed-industries/zed) for reference. Not part of the locus.codes build.

## Source

- **Repo:** https://github.com/zed-industries/zed  
- **Path:** `crates/theme/` (main branch)

## Layout

| File | Purpose |
|------|--------|
| `crates/theme/src/default_colors.rs` | Default color definitions (~68KB) — main theme palette |
| `crates/theme/src/fallback_themes.rs` | Fallback theme definitions |
| `crates/theme/src/theme.rs` | Theme types, refinement, `GlobalTheme` |
| `crates/theme/src/schema.rs` | Theme JSON schema |
| `crates/theme/src/settings.rs` | Theme settings and overrides |
| `crates/theme/src/registry.rs` | Theme registry / loading |
| `crates/theme/src/scale.rs` | Color scale utilities |
| `crates/theme/src/styles/*.rs` | accents, colors, players, status, syntax, system |
| `crates/theme/src/icon_theme*.rs` | Icon theme types and schema |
| `crates/theme/src/font_family_cache.rs` | Font family cache |

## Updating

To refresh from upstream:

```bash
BASE="https://raw.githubusercontent.com/zed-industries/zed/main"
ROOT="zed_default_theme"
for path in crates/theme/Cargo.toml crates/theme/src/*.rs crates/theme/src/styles/*.rs; do
  mkdir -p "$ROOT/$(dirname $path)"
  curl -sL "$BASE/$path" -o "$ROOT/$path"
done
```

## License

Zed is licensed under GPL-3.0 (and others). See [zed-industries/zed](https://github.com/zed-industries/zed) for details. This copy is for reference only.
