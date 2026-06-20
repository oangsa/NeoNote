---
name: NeoNote
description: A native, modern, precise Windows note-taking app with in-process Vim editing.
colors:
  background: "#0f1117"
  background-alt: "#161922"
  surface: "#1d2430"
  border: "#303848"
  text: "#d7dae0"
  text-muted: "#7f8490"
  accent-primary: "#7aa2f7"
  accent-secondary: "#c678dd"
  success: "#98c379"
  warning: "#e5c07b"
  error: "#e06c75"
  cursor: "#d7dae0"
typography:
  title:
    fontFamily: "system-ui, Segoe UI, sans-serif"
    fontSize: "18px"
    fontWeight: 700
    lineHeight: 1.2
    letterSpacing: "0"
  body:
    fontFamily: "system-ui, Segoe UI, sans-serif"
    fontSize: "13px"
    fontWeight: 500
    lineHeight: 1.35
    letterSpacing: "0"
  label:
    fontFamily: "system-ui, Segoe UI, sans-serif"
    fontSize: "11px"
    fontWeight: 600
    lineHeight: 1.25
    letterSpacing: "0"
  editor:
    fontFamily: "JetBrains Mono, Cascadia Code, Fira Code, Consolas, monospace"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.4
    letterSpacing: "0"
rounded:
  none: "0px"
  xs: "2px"
  sm: "4px"
  md: "6px"
  lg: "8px"
  pill: "10px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
  xl: "20px"
components:
  command-button:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    rounded: "{rounded.md}"
    height: "30px"
    padding: "0 12px"
  command-button-hover:
    backgroundColor: "{colors.border}"
    textColor: "{colors.text}"
    rounded: "{rounded.md}"
    height: "30px"
    padding: "0 12px"
  toggle-track-on:
    backgroundColor: "{colors.accent-primary}"
    rounded: "{rounded.pill}"
    width: "38px"
    height: "20px"
  dialog-panel:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    rounded: "{rounded.lg}"
  editor-cursor:
    backgroundColor: "{colors.cursor}"
    rounded: "{rounded.none}"
---

# Design System: NeoNote

## 1. Overview

**Creative North Star: "Fluent Text Workshop"**

NeoNote is a native Windows work surface for text: calm enough for long writing sessions, precise enough for Vim-style editing, and polished enough to feel like a finished productivity app. The system should feel tactile and native, with controls that respond immediately and surfaces that sit naturally inside a Windows desktop environment.

The interface is restrained because the editor owns the experience. Chrome, tabs, settings, status text, and theme controls should support writing without becoming a dashboard. NeoNote rejects Electron, WebView, web dashboard, browser-based editor, developer-tool prototype, heavy Obsidian-style vault, Neovide particle-effects clone, and generic SaaS interface patterns.

**Key Characteristics:**
- Native Windows productivity feel, not web-app decoration.
- Dark and light theme support through the same semantic theme tokens.
- Dense but readable controls built around 13px labels and a 14px mono editor.
- Subtle motion for state feedback, never page choreography.
- Dialogs and popups use native lifted elevation; editor and chrome stay mostly flat.

## 2. Colors

NeoNote uses theme-driven semantic colors. The NeoVim Dark palette is the default reference, but every active theme must map into the same roles so existing themes remain compatible.

### Primary
- **Command Blue** (`accent-primary`): Used for primary actions, active tab accents, search counters, preview state, selection fill, and current navigation cues. It should remain rare enough to feel meaningful.

### Secondary
- **Visual Violet** (`accent-secondary`): Used as a secondary accent and hover alternative for primary action surfaces. It should support Vim visual-mode energy without becoming decorative.

### Tertiary
- **Mode Semantics** (`success`, `warning`, `error`): Used for insert/success, command/warning, replace/error, status feedback, and destructive hover states. These colors must remain semantic, not ornamental.

### Neutral
- **Editor Night** (`background`): The main editor and app background.
- **Chrome Night** (`background-alt`): Title bar, status bar, sidebars, and secondary chrome.
- **Raised Slate** (`surface`): Dialogs, menus, active tabs, settings rows, and card-like panels.
- **Control Stroke** (`border`): Dividers, button outlines, row borders, and inactive theme accents.
- **Primary Ink** (`text`): Main UI text and editor content.
- **Muted Ink** (`text-muted`): Secondary labels, inactive tabs, status text, and metadata.
- **Caret Ink** (`cursor`): Cursor surface. In block mode it appears as a translucent block over editor text.

### Named Rules

**The Existing Themes Rule.** New UI must consume `theme-background`, `theme-background-alt`, `theme-surface`, `theme-border`, `theme-text`, `theme-text-muted`, and semantic accent tokens. Never hardcode a one-off palette into active UI.

**The Accent Rarity Rule.** Accent color is for current state, primary action, cursor/search affordances, and mode information. Do not use it as general decoration.

**The Semantic State Rule.** Success, warning, and error colors are state colors. They must not become category colors, badges, or visual confetti.

## 3. Typography

**Display Font:** system UI, Segoe UI, sans-serif
**Body Font:** system UI, Segoe UI, sans-serif
**Label/Mono Font:** JetBrains Mono with Cascadia Code, Fira Code, Consolas, monospace fallbacks for the editor

**Character:** Product UI should use familiar Windows typography with no display theatrics. The editor is mono-forward and precise; surrounding UI is compact, readable, and quiet.

### Hierarchy
- **Display** (not used): NeoNote does not use hero-scale display type in the app shell.
- **Headline** (700, 18px, 1.2): Dialog titles such as Settings and Themes.
- **Title** (700, 15-16px, 1.25): Section headings, app title, and grouped settings labels.
- **Body** (500-600, 13px, 1.35): Buttons, settings row titles, menu items, and primary UI labels.
- **Label** (600-700, 11-12px, 1.25): Status bar mode labels, metadata, theme badges, and detail text.
- **Editor** (400, 14px, 1.4): Note content, line text, command/search interaction, and cursor measurement.

### Named Rules

**The No Hero Type Rule.** NeoNote is a working editor, not a landing page. Large marketing typography is forbidden inside the product surface.

**The Mono Precision Rule.** Text editing, cursor measurement, and selection rendering must stay anchored to the configured monospace editor font.

## 4. Elevation

NeoNote uses Native lifted elevation: flat tonal layers for the app shell and editor, with clear lifted treatment only for transient surfaces such as popups, dialogs, menus, and overlays. Depth should feel like Windows productivity software, not glassmorphism or web dashboard cards.

### Shadow Vocabulary
- **Popup Lift** (`drop-shadow-color: #000000.with_alpha(0.3); drop-shadow-blur: 12px; drop-shadow-offset-y: 4px`): File menus and lightweight popups.
- **Dialog Lift** (`drop-shadow-color: #000000.with_alpha(0.25); drop-shadow-blur: 16px; drop-shadow-offset-y: 6px`): Settings and theme dialogs.
- **Overlay Dim** (`theme-background.with_alpha(0.68)`): Modal backdrop that focuses the dialog without creating a heavy theatrical fade.

### Named Rules

**The Native Lift Rule.** Resting chrome is flat or tonal. Only popups and dialogs lift.

**The No Ghost Card Rule.** Do not pair decorative wide soft shadows with bordered card grids. NeoNote uses borders for structure and shadows for actual transient elevation.

## 5. Components

### Buttons

Buttons are tactile and native: compact, bordered, and stateful without looking web-heavy.

- **Shape:** Gently rounded rectangles (6px radius).
- **Primary:** Accent background with background-colored text for apply/save-style actions.
- **Secondary:** Surface or background-alt fill, border stroke, muted or primary text.
- **Hover / Focus:** Background and border shift over 120-160ms with ease-out. Add focus treatment before shipping keyboard-heavy surfaces.
- **Sizing:** Default command height is 30px; width should be explicit for toolbar and dialog alignment.

### Chips

Theme variant badges act as compact chips.

- **Style:** Background-alt fill, 1px border at low opacity, 4px radius, 11px muted label.
- **State:** Active and preview states are not chip-only; they also affect row border, row background, and status text.

### Cards / Containers

Containers are used for repeated list rows, dialogs, and transient panels rather than for every section.

- **Corner Style:** Rows use 6-8px; dialogs use 8px.
- **Background:** Surface for lifted panels; background or background-alt for rows.
- **Shadow Strategy:** Dialog Lift or Popup Lift only.
- **Border:** 1px border using `theme-border`, often with alpha for quiet separation.
- **Internal Padding:** 12-20px depending on density.

### Inputs / Fields

The active codebase uses numeric steppers and action rows rather than text input fields in Settings.

- **Style:** Controls should use the same 6px radius, 1px border, and surface/background-alt fills as command buttons.
- **Focus:** Keyboard focus must be visible and should use the primary accent without changing layout.
- **Disabled:** Disabled controls should keep structure visible while muting text and reducing contrast carefully enough to remain readable.

### Navigation

Navigation appears as tabs, settings sidebar items, and menu commands.

- **Tabs:** Active tab uses background fill plus a 2px accent underline. Inactive tabs remain quiet; hover uses a tonal background without height change.
- **Settings sidebar:** Active section uses a tonal active background and a narrow internal indicator. Inactive sections are muted but readable.
- **Menus:** Menus are lifted popups with compact command rows and 6px radius.
- **Status bar:** Status communicates mode, file, cursor position, line count, word count, encoding, and search count without becoming noisy.

### Editor Surface

The editor is the signature component.

- **Line layout:** Gutter width, content padding, line height, and cursor metrics are shared by selection, cursor, and pointer math.
- **Selection:** Rust emits grouped selection path data; Slint scales it behind text using theme accent at low alpha.
- **Cursor:** Block cursor is translucent over text; non-block cursor uses mode color. Insert responsiveness wins over animation.
- **Search:** Matches use warning color at low alpha; count appears in the status bar using primary accent.

## 6. Do's and Don'ts

### Do:

- **Do** preserve existing themes by styling through semantic theme tokens, not direct palette values.
- **Do** keep the editor content first; chrome should be quiet, compact, and supportive.
- **Do** use Native lifted elevation for dialogs and popups only.
- **Do** keep motion purposeful: hover, dialog open/close, tab transitions, cursor glide, smooth scroll, and theme transitions.
- **Do** verify muted text contrast across both dark and light built-in themes.
- **Do** keep controls keyboard-accessible with visible focus states.
- **Do** treat Vim mode colors as state feedback.

### Don't:

- **Don't** make NeoNote look like an Electron, WebView, web dashboard, or browser-based editor.
- **Don't** ship developer-tool prototype spacing, inconsistent controls, or unpolished dialog surfaces.
- **Don't** add heavy Obsidian-style vault, workspace, plugin system, file explorer, Markdown preview, or split-pane knowledge base UI.
- **Don't** add Neovide particle effects, sparkles, ripples, GPU-heavy cursor effects, or distracting motion.
- **Don't** use generic SaaS interface patterns such as overdesigned cards, decorative gradients, or web-first interaction patterns.
- **Don't** use side-stripe card accents. A narrow active indicator inside navigation is allowed; colored border-left decoration on content cards is not.
- **Don't** use gradient text, glassmorphism as a default material, or hero-metric templates.
- **Don't** animate layout in ways that make typing, cursor movement, or scrolling feel delayed.
