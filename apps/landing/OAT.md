# Oat UI setup (oat.ink)

This project uses [Oat](https://oat.ink) following the official docs.

## Usage

- **Install:** `@knadh/oat` (see `package.json`).
- **Load order** in `src/main.tsx`:
  1. `@knadh/oat/oat.min.css` – Oat base + components
  2. `@knadh/oat/oat.min.js` – WebComponents (dialog, dropdown, tabs, toast)
  3. `src/oat-theme.css` – our theme override (must be after Oat CSS)
  4. `src/index.css` – layout and page-specific styles

Ref: [oat.ink/usage](https://oat.ink/usage/)

## Customizing

Theme variables are overridden in **`src/oat-theme.css`** (loaded after Oat). We set:

- `--background`, `--foreground`, `--primary`, `--muted`, `--border`, etc. for light/dark.
- `--font-sans` to Geist Pixel.
- Dark mode via `[data-theme="dark"]` on `<body>`.

Ref: [oat.ink/customizing](https://oat.ink/customizing/)

## Components

We use Oat’s semantic patterns:

- **Buttons:** `<button>` or `<a class="button">` – styled by Oat.
- **Switch:** `<input type="checkbox" role="switch">` – we wrap it in a custom control; Oat styles the switch.
- **Typography:** `<h1>`, `<h2>`, `<p>`, `<strong>` – Oat styles semantic HTML.
- **Layout:** Our own layout (full-height sections); Oat provides `.container`, `.row`, `.col-*` if needed.

Ref: [oat.ink/components](https://oat.ink/components/)
