# Top bar: locked design

Approved 2026-10-26. `top-bar-preview.html` (open it in a browser) is the reference; the native bar
(`src/bin/bar.rs`) is drawn to match it. Change this file and the preview first, then the code.

## Bar
- Always dark, in light and dark Windows mode alike.
- Background `#0D0807`, corner radius **10**, inner padding **10** on every side.
- 378 × 48 at 100% scaling; everything scales with the display.
- Left to right: logo · 12 gap · switch · (space between) · four icons, 16 apart.

## Logo
- `Resources/nu-icon.svg`, 37 × 28, radius 6, gradient `#D13A0E` → `#E87A1E`, white glyph at 90%.
- Drag handle for the bar. A click toggles English ⇄ বাংলা.

## Switch (dropdown)
- 161 × 28, radius 6, background `#241C1A`, 1px border `#2D2827`, padding 4 / 8.
- Text 15px Aptos (Segoe UI if Aptos is missing), white: `English`, `বাংলা · বিজয়` or
  `বাংলা · ফোনেটিক`. The box looks the same in both states.
- Lucide `chevron-down` at the right. A click opens the layout menu.

## Icons
- Lucide, 20px, 1px stroke, white at 90%, round caps and joins.
- `settings` → settings window · `info` → help menu · to-tray (bar over a downward triangle) →
  hides the bar to the tray · `x` → menu: send to tray, or exit.
- Hover: a 28px rounded square in `#241C1A` behind the icon; `#C42B1C` behind `x`.

## Menus
- Custom, drawn in the bar's style: background `#0D0807`, 1px border `#2D2827`, radius 8,
  padding 6; items 32 tall, radius 6, hover `#241C1A`; separators `#2D2827`.
- Lucide icons at 16px; a `check` marks the current layout; shortcuts (F12) in dim text.
- Layout menu: English / বাংলা · বিজয় / বাংলা · ফোনেটিক (on and off are F12, the logo and the tray icon).
- Help menu: ব্যবহারের নির্দেশিকা (nuktasolutions.com/nukta-bangla-keyboard/guide), ওয়েবসাইটে যান
  (nuktasolutions.com/nukta-bangla-keyboard) — নুকতা বাংলা সম্পর্কে.
- Tray menu: toggle (F12), show / hide the top bar — কীবোর্ড লেআউট ▸, সেটিংস… — সাহায্য ▸ — বন্ধ করুন.

## Tray icon
- A rounded square. **On**: the brand gradient (`#D13A0E` → `#E87A1E`) with the নু glyph from
  `nu-icon.svg`. **Off**: our black `#0D0807` with a faint `#4A413F` edge and white **EN**.
- Off is completely off: the keyboard leaves every key to the app, as Windows' own English
  keyboard would; only F12 is noticed, to turn it back on.
- Click: toggles (after the double-click time, so a double-click doesn't toggle).
- Double-click: shows the top bar again at its default spot (top right).
- Right click: the tray menu.
- No Windows language indicator (EN) and no keyboard icon from Windows in the taskbar: this icon,
  the top bar and F12 are the only switches.

## Never in any UI
- The name of any competing keyboard.
