#![allow(dead_code)]

mod app;
mod notes;
mod persistence;
mod platform;
mod theme;

use std::{cell::RefCell, rc::Rc};

use app::{AppController, AppSnapshot};
use slint::{Brush, Color, ComponentHandle, SharedString, VecModel};

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let window = AppWindow::new()?;
    let controller = Rc::new(RefCell::new(AppController::new()));

    install_callbacks(&window, Rc::clone(&controller));
    apply_snapshot(&window, &controller.borrow().snapshot());

    window.run()
}

fn install_callbacks(window: &AppWindow, controller: Rc<RefCell<AppController>>) {
    let weak_window = window.as_weak();
    let controller_for_new = Rc::clone(&controller);
    window.on_new_file(move || {
        controller_for_new.borrow_mut().new_file();
        if let Some(window) = weak_window.upgrade() {
            apply_snapshot(&window, &controller_for_new.borrow().snapshot());
        }
    });

    let weak_window = window.as_weak();
    let controller_for_open = Rc::clone(&controller);
    window.on_open_file(move || {
        controller_for_open.borrow_mut().open_file_dialog();
        if let Some(window) = weak_window.upgrade() {
            apply_snapshot(&window, &controller_for_open.borrow().snapshot());
        }
    });

    let weak_window = window.as_weak();
    let controller_for_save = Rc::clone(&controller);
    window.on_save_file(move || {
        controller_for_save.borrow_mut().save();
        if let Some(window) = weak_window.upgrade() {
            apply_snapshot(&window, &controller_for_save.borrow().snapshot());
        }
    });

    let weak_window = window.as_weak();
    let controller_for_save_as = Rc::clone(&controller);
    window.on_save_as_file(move || {
        controller_for_save_as.borrow_mut().save_as();
        if let Some(window) = weak_window.upgrade() {
            apply_snapshot(&window, &controller_for_save_as.borrow().snapshot());
        }
    });

    let weak_window = window.as_weak();
    let controller_for_previous = Rc::clone(&controller);
    window.on_previous_document(move || {
        controller_for_previous.borrow_mut().previous_document();
        if let Some(window) = weak_window.upgrade() {
            apply_snapshot(&window, &controller_for_previous.borrow().snapshot());
            window.invoke_focus_editor();
        }
    });

    let weak_window = window.as_weak();
    let controller_for_next = Rc::clone(&controller);
    window.on_next_document(move || {
        controller_for_next.borrow_mut().next_document();
        if let Some(window) = weak_window.upgrade() {
            apply_snapshot(&window, &controller_for_next.borrow().snapshot());
            window.invoke_focus_editor();
        }
    });

    let weak_window = window.as_weak();
    let controller_for_theme = Rc::clone(&controller);
    window.on_open_theme_panel(move || {
        controller_for_theme.borrow_mut().open_theme_panel();
        if let Some(window) = weak_window.upgrade() {
            apply_snapshot(&window, &controller_for_theme.borrow().snapshot());
        }
    });

    let weak_window = window.as_weak();
    let controller_for_settings = Rc::clone(&controller);
    window.on_open_settings_panel(move || {
        controller_for_settings.borrow_mut().open_settings_panel();
        if let Some(window) = weak_window.upgrade() {
            apply_snapshot(&window, &controller_for_settings.borrow().snapshot());
        }
    });

    let weak_window = window.as_weak();
    let controller_for_editor = Rc::clone(&controller);
    window.on_editor_key(move |key| {
        controller_for_editor
            .borrow_mut()
            .handle_editor_key(key.as_str());
        if let Some(window) = weak_window.upgrade() {
            apply_editor_snapshot(&window, &controller_for_editor.borrow().snapshot());
            window.invoke_focus_editor();
        }
    });
}

fn apply_snapshot(window: &AppWindow, snapshot: &AppSnapshot) {
    apply_editor_snapshot(window, snapshot);
    window.set_theme_background(brush_from_hex(&snapshot.theme.background));
    window.set_theme_background_alt(brush_from_hex(&snapshot.theme.background_alt));
    window.set_theme_surface(brush_from_hex(&snapshot.theme.surface));
    window.set_theme_border(brush_from_hex(&snapshot.theme.border));
    window.set_theme_text(brush_from_hex(&snapshot.theme.text));
    window.set_theme_text_muted(brush_from_hex(&snapshot.theme.text_muted));
    window.set_theme_accent_primary(brush_from_hex(&snapshot.theme.accent_primary));
    window.set_theme_accent_secondary(brush_from_hex(&snapshot.theme.accent_secondary));
    window.set_theme_success(brush_from_hex(&snapshot.theme.success));
    window.set_theme_warning(brush_from_hex(&snapshot.theme.warning));
    window.set_theme_error(brush_from_hex(&snapshot.theme.error));
    window.set_theme_cursor(brush_from_hex(&snapshot.theme.cursor));
}

fn apply_editor_snapshot(window: &AppWindow, snapshot: &AppSnapshot) {
    window.set_file_title(SharedString::from(snapshot.file_title.as_str()));
    window.set_file_path(SharedString::from(snapshot.file_path.as_str()));
    window.set_editor_lines(
        Rc::new(VecModel::from(
            snapshot
                .editor_lines
                .iter()
                .map(|line| EditorLine {
                    number: line.number,
                    text: SharedString::from(line.text.as_str()),
                    is_cursor_line: line.is_cursor_line,
                    cursor_column: line.cursor_column,
                    cursor_block: line.cursor_block,
                })
                .collect::<Vec<_>>(),
        ))
        .into(),
    );
    window.set_document_tabs(SharedString::from(snapshot.document_tabs.as_str()));
    window.set_status_text(SharedString::from(snapshot.status_text.as_str()));
    window.set_status_right(SharedString::from(snapshot.status_right.as_str()));
    window.set_mode_text(SharedString::from(snapshot.mode_text.as_str()));
    window.set_cursor_line(snapshot.cursor_line);
    window.set_cursor_column(snapshot.cursor_column);
    window.set_cursor_prefix(SharedString::from(snapshot.cursor_prefix.as_str()));
    window.set_cursor_cell(SharedString::from(snapshot.cursor_cell.as_str()));
    window.set_cursor_suffix(SharedString::from(snapshot.cursor_suffix.as_str()));
    window.set_cursor_block(snapshot.cursor_block);
    window.set_message(SharedString::from(snapshot.message.as_str()));
    window.set_has_document(snapshot.has_document);
    window.set_editor_font_family(SharedString::from(snapshot.editor_font_family.as_str()));
    window.set_mode_color(brush_from_hex(&snapshot.mode_color));
}

fn brush_from_hex(value: &str) -> Brush {
    let fallback = Color::from_rgb_u8(122, 162, 247);
    let Some(hex) = value.strip_prefix('#') else {
        return fallback.into();
    };

    if hex.len() != 6 {
        return fallback.into();
    }

    let parse = |range: std::ops::Range<usize>| u8::from_str_radix(&hex[range], 16).ok();
    match (parse(0..2), parse(2..4), parse(4..6)) {
        (Some(red), Some(green), Some(blue)) => Color::from_rgb_u8(red, green, blue).into(),
        _ => fallback.into(),
    }
}
