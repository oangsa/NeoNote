# Product

## Register

product

## Users

NeoNote is primarily for the owner-builder and for Windows note-takers who want a fast, native, keyboard-first notes app. Users are often writing or editing plain text notes, work logs, quick records, and project thoughts while staying in flow on Windows.

The app should also serve users who like Vim-style editing but do not want to configure or launch an external editor, terminal, browser renderer, or full workspace system just to write notes.

## Product Purpose

NeoNote exists to make focused note-taking feel native, immediate, and precise on Windows. It combines a Slint-based Windows desktop shell with an in-process Rust Vim-style editor, native file dialogs, tabs, sessions, recent files, and themes.

Success means a user can open NeoNote, start writing, move through text with familiar Vim behavior, and trust the application as a polished daily tool. It should feel production-ready without becoming a vault, IDE, plugin host, or browser-style knowledge workspace.

## Brand Personality

Native, modern, precise.

NeoNote should feel calm, crafted, lightweight, and professional. The interface should communicate quality through restraint, clear hierarchy, comfortable spacing, and fast feedback rather than decorative complexity.

## Anti-references

NeoNote should not look or feel like:

- An Electron, WebView, web dashboard, or browser-based editor.
- A developer-tool prototype with rough spacing, inconsistent controls, or unpolished dialog surfaces.
- A heavy Obsidian-style vault, workspace, plugin system, file explorer, Markdown preview, or split-pane knowledge base.
- A Neovide or particle-effects clone with GPU-heavy cursor effects, sparkles, ripples, or distracting motion.
- A generic SaaS interface with overdesigned cards, decorative gradients, or web-first interaction patterns.

Good reference quality targets include modern Windows Notepad, VS Code, Obsidian, JetBrains IDEs, Microsoft Loop, and polished native Windows productivity software, with the caveat that NeoNote must remain simpler, lighter, and more focused than most of those products.

## Design Principles

1. Content first. The editor is the primary experience; surrounding chrome should support writing and navigation without stealing attention.
2. Native by default. Controls, window behavior, dialogs, motion, spacing, and feedback should feel at home on modern Windows.
3. Precision over spectacle. Vim behavior, cursor rendering, scrolling, selection, and status information should feel exact and trustworthy.
4. Calm polish. Use subtle animation, theme transitions, hover states, and Mica-style surfaces only when they improve perceived quality and responsiveness.
5. Lightweight boundaries. Avoid feature expansion that turns NeoNote into a workspace, vault, browser app, or plugin platform.

## Accessibility & Inclusion

NeoNote should remain usable without a mouse and should preserve keyboard-first workflows. Interactive controls need clear hover, focus, active, and disabled states. Text, muted labels, status information, selections, and cursor states should maintain sufficient contrast across existing built-in and user themes.

Motion must be subtle, purposeful, and disableable. Animation settings should apply immediately where possible, and editor responsiveness always wins over visual effects. Font size, line height, theme choice, and window opacity should remain adjustable for comfort.
