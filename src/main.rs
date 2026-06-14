#![allow(dead_code)]

mod app;
mod notes;
mod persistence;
mod platform;
mod theme;
mod vim;

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
    let controller_for_theme_preview = Rc::clone(&controller);
    window.on_theme_preview(move |index| {
        controller_for_theme_preview
            .borrow_mut()
            .preview_theme(index);
        if let Some(window) = weak_window.upgrade() {
            apply_snapshot(&window, &controller_for_theme_preview.borrow().snapshot());
        }
    });

    let weak_window = window.as_weak();
    let controller_for_theme_apply = Rc::clone(&controller);
    window.on_theme_apply(move |index| {
        controller_for_theme_apply.borrow_mut().apply_theme(index);
        if let Some(window) = weak_window.upgrade() {
            apply_snapshot(&window, &controller_for_theme_apply.borrow().snapshot());
            window.invoke_focus_editor();
        }
    });

    let weak_window = window.as_weak();
    let controller_for_theme_close = Rc::clone(&controller);
    window.on_theme_panel_close(move || {
        controller_for_theme_close.borrow_mut().close_theme_panel();
        if let Some(window) = weak_window.upgrade() {
            apply_snapshot(&window, &controller_for_theme_close.borrow().snapshot());
            window.invoke_focus_editor();
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
    let controller_for_settings_close = Rc::clone(&controller);
    window.on_settings_panel_close(move || {
        controller_for_settings_close
            .borrow_mut()
            .close_settings_panel();
        if let Some(window) = weak_window.upgrade() {
            apply_snapshot(&window, &controller_for_settings_close.borrow().snapshot());
            window.invoke_focus_editor();
        }
    });

    let weak_window = window.as_weak();
    let controller_for_font_size = Rc::clone(&controller);
    window.on_settings_adjust_font_size(move |delta| {
        controller_for_font_size
            .borrow_mut()
            .adjust_font_size(delta);
        if let Some(window) = weak_window.upgrade() {
            apply_snapshot(&window, &controller_for_font_size.borrow().snapshot());
        }
    });

    let weak_window = window.as_weak();
    let controller_for_line_height = Rc::clone(&controller);
    window.on_settings_adjust_line_height(move |delta| {
        controller_for_line_height
            .borrow_mut()
            .adjust_line_height(delta);
        if let Some(window) = weak_window.upgrade() {
            apply_snapshot(&window, &controller_for_line_height.borrow().snapshot());
        }
    });

    let weak_window = window.as_weak();
    let controller_for_tab_size = Rc::clone(&controller);
    window.on_settings_adjust_tab_size(move |delta| {
        controller_for_tab_size.borrow_mut().adjust_tab_size(delta);
        if let Some(window) = weak_window.upgrade() {
            apply_snapshot(&window, &controller_for_tab_size.borrow().snapshot());
        }
    });

    let weak_window = window.as_weak();
    let controller_for_opacity = Rc::clone(&controller);
    window.on_settings_adjust_window_opacity(move |delta| {
        controller_for_opacity
            .borrow_mut()
            .adjust_window_opacity(delta);
        if let Some(window) = weak_window.upgrade() {
            apply_snapshot(&window, &controller_for_opacity.borrow().snapshot());
        }
    });

    let weak_window = window.as_weak();
    let controller_for_word_wrap = Rc::clone(&controller);
    window.on_settings_toggle_word_wrap(move || {
        controller_for_word_wrap.borrow_mut().toggle_word_wrap();
        if let Some(window) = weak_window.upgrade() {
            apply_snapshot(&window, &controller_for_word_wrap.borrow().snapshot());
        }
    });

    let weak_window = window.as_weak();
    let controller_for_clipboard = Rc::clone(&controller);
    window.on_settings_toggle_sync_clipboard(move || {
        controller_for_clipboard
            .borrow_mut()
            .toggle_sync_clipboard();
        if let Some(window) = weak_window.upgrade() {
            apply_snapshot(&window, &controller_for_clipboard.borrow().snapshot());
        }
    });

    let weak_window = window.as_weak();
    let controller_for_restore = Rc::clone(&controller);
    window.on_settings_toggle_restore_last_session(move || {
        controller_for_restore
            .borrow_mut()
            .toggle_restore_last_session();
        if let Some(window) = weak_window.upgrade() {
            apply_snapshot(&window, &controller_for_restore.borrow().snapshot());
        }
    });

    let weak_window = window.as_weak();
    let controller_for_launcher = Rc::clone(&controller);
    window.on_settings_toggle_show_launcher_on_startup(move || {
        controller_for_launcher
            .borrow_mut()
            .toggle_show_launcher_on_startup();
        if let Some(window) = weak_window.upgrade() {
            apply_snapshot(&window, &controller_for_launcher.borrow().snapshot());
        }
    });

    let weak_window = window.as_weak();
    let controller_for_geometry = Rc::clone(&controller);
    window.on_settings_toggle_remember_window_geometry(move || {
        controller_for_geometry
            .borrow_mut()
            .toggle_remember_window_geometry();
        if let Some(window) = weak_window.upgrade() {
            apply_snapshot(&window, &controller_for_geometry.borrow().snapshot());
        }
    });

    let weak_window = window.as_weak();
    let controller_for_blur = Rc::clone(&controller);
    window.on_settings_toggle_blur_behind(move || {
        controller_for_blur.borrow_mut().toggle_blur_behind();
        if let Some(window) = weak_window.upgrade() {
            apply_snapshot(&window, &controller_for_blur.borrow().snapshot());
        }
    });

    let weak_window = window.as_weak();
    let controller_for_editor = Rc::clone(&controller);
    window.on_editor_key(move |key| {
        controller_for_editor
            .borrow_mut()
            .handle_editor_key(key.as_str());

        let has_highlight = {
            let ctrl = controller_for_editor.borrow();
            let note = ctrl.active_note();
            note.has_yank_highlight() || note.has_deferred_action()
        };

        if let Some(window) = weak_window.upgrade() {
            apply_editor_snapshot(&window, &controller_for_editor.borrow().snapshot());
            window.invoke_focus_editor();

            if has_highlight {
                let weak_window_inner = window.as_weak();
                let controller_inner = Rc::clone(&controller_for_editor);
                slint::Timer::single_shot(std::time::Duration::from_millis(200), move || {
                    if let Some(w) = weak_window_inner.upgrade() {
                        let mut ctrl = controller_inner.borrow_mut();
                        ctrl.flush_deferred_action();
                        ctrl.active_note_mut().clear_yank_highlight();
                        apply_editor_snapshot(&w, &ctrl.snapshot());
                    }
                });
            }
        }
    });

    let weak_window = window.as_weak();
    let controller_for_pointer = Rc::clone(&controller);
    window.on_editor_pointer_event(move |line, x_pixels, event_kind| {
        controller_for_pointer.borrow_mut().handle_editor_pointer(
            line,
            x_pixels,
            event_kind.as_str(),
        );
        if let Some(window) = weak_window.upgrade() {
            apply_editor_snapshot(&window, &controller_for_pointer.borrow().snapshot());
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
    window.set_show_theme_panel(snapshot.theme_panel_open);
    window.set_show_settings_panel(snapshot.settings_panel_open);
    window.set_settings(SettingsData {
        font_family: SharedString::from(snapshot.settings.font_family.as_str()),
        font_size: snapshot.settings.font_size,
        font_size_label: SharedString::from(snapshot.settings.font_size_label.as_str()),
        line_height: snapshot.settings.line_height,
        line_height_label: SharedString::from(snapshot.settings.line_height_label.as_str()),
        tab_size: snapshot.settings.tab_size,
        tab_size_label: SharedString::from(snapshot.settings.tab_size_label.as_str()),
        word_wrap: snapshot.settings.word_wrap,
        sync_clipboard: snapshot.settings.sync_clipboard,
        restore_last_session: snapshot.settings.restore_last_session,
        show_launcher_on_startup: snapshot.settings.show_launcher_on_startup,
        remember_window_geometry: snapshot.settings.remember_window_geometry,
        blur_behind: snapshot.settings.blur_behind,
        window_opacity: snapshot.settings.window_opacity,
        window_opacity_label: SharedString::from(snapshot.settings.window_opacity_label.as_str()),
    });
    window.set_theme_items(
        Rc::new(VecModel::from(
            snapshot
                .theme_items
                .iter()
                .map(|item| ThemeItem {
                    index: item.index,
                    name: SharedString::from(item.name.as_str()),
                    variant: SharedString::from(item.variant.as_str()),
                    author: SharedString::from(item.author.as_str()),
                    is_active: item.is_active,
                    is_preview: item.is_preview,
                })
                .collect::<Vec<_>>(),
        ))
        .into(),
    );
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
                    selected_prefix: SharedString::from(line.selected_prefix.as_str()),
                    selected_text: SharedString::from(line.selected_text.as_str()),
                    is_search_match: line.is_search_match,
                    cursor_column: line.cursor_column,
                    cursor_prefix: SharedString::from(line.cursor_prefix.as_str()),
                    cursor_cell: SharedString::from(line.cursor_cell.as_str()),
                    cursor_suffix: SharedString::from(line.cursor_suffix.as_str()),
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
    window.set_editor_font_size(snapshot.editor_font_size);
    window.set_editor_line_height(snapshot.editor_line_height);
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
