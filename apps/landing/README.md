# locus.codes landing

Landing page for [locus.codes](https://locus.codes) — frontier coding agent with memory.

## Stack

- **React** + **TypeScript** (Vite)
- **Oat** ([oat.ink](https://oat.ink)) — minimal semantic UI (CSS + JS)
- **Geist Pixel** — [Vercel font](https://vercel.com/font?type=pixel)
- **Buttondown** — early access email (newsletter: `fnlog0`)

## Run

```bash
npm install
npm run dev
```

Open [http://localhost:5173](http://localhost:5173).

## Scripts

| Command        | Description              |
|----------------|--------------------------|
| `npm run dev`  | Start dev server         |
| `npm run build`| Production build         |
| `npm run preview` | Preview production build |
| `npm run lint` | Run ESLint               |

## Structure

```
landing/
├── public/
│   ├── locus.svg      # Favicon
│   └── fonts/         # Geist Pixel woff2
├── src/
│   ├── css/           # Split styles
│   │   ├── main.css   # Imports all
│   │   ├── base.css
│   │   ├── background.css
│   │   ├── nav.css
│   │   ├── hero.css
│   │   ├── section.css
│   │   ├── get-cta.css
│   │   └── theme-switch.css
│   ├── components/    # React components
│   ├── hooks/         # useTheme
│   ├── App.tsx
│   ├── main.tsx
│   └── oat-theme.css   # Oat variable overrides
├── OAT.md              # Oat setup (usage, customizing, components)
└── README.md
```

## Features

- **Light / dark theme** — toggle in nav; default is light; preference stored in `localStorage`
- **Early access** — inline form posts to Buttondown (`fnlog0`); see `GetCTA.tsx` for `BUTTONDOWN_EMBED_URL`
- **Full-height layout** — no scroll; hero, description, and CTA fill the viewport

## Oat

Theme and components follow [oat.ink](https://oat.ink). See **[OAT.md](./OAT.md)** for usage, customizing, and component notes.
