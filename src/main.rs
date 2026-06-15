#![allow(dead_code)]
#![windows_subsystem = "windows"]

mod app;
mod i18n;
mod notes;
mod persistence;
mod platform;
mod theme;
mod vim;

mod startup;

use std::{cell::RefCell, rc::Rc};

use app::{AppController, AppSnapshot};
use slint::{Brush, Color, ComponentHandle, SharedString, VecModel};

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let startup_args = startup::parse_startup_args();

    #[cfg(target_os = "windows")]
    {
        let msg = if !startup_args.files_to_open.is_empty() {
            platform::ipc::IpcMessage::OpenFiles {
                files: startup_args.files_to_open.clone(),
            }
        } else {
            platform::ipc::IpcMessage::FocusWindow
        };

        if platform::ipc::try_send_ipc_message(&msg).is_ok() {
            // Forwarded to existing instance successfully.
            return Ok(());
        }
    }

    let window = AppWindow::new()?;
    let controller = Rc::new(RefCell::new(AppController::new()));

    controller.borrow_mut().run_startup_flow(startup_args);

    install_callbacks(&window, Rc::clone(&controller));
    apply_snapshot(&window, &controller.borrow().snapshot());

    #[cfg(target_os = "windows")]
    let _ipc_timer = {
        let weak_window = window.as_weak();
        let ipc_controller = Rc::clone(&controller);
        
        let (tx, rx) = std::sync::mpsc::channel();
        
        platform::ipc::start_ipc_server(move |msg| {
            let _ = tx.send(msg);
        });

        let timer = slint::Timer::default();
        timer.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(100), move || {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    platform::ipc::IpcMessage::OpenFiles { files } => {
                        ipc_controller.borrow_mut().open_files(files);
                    }
                    platform::ipc::IpcMessage::FocusWindow => {
                        // Focus if possible
                    }
                }
                if let Some(w) = weak_window.upgrade() {
                    apply_snapshot(&w, &ipc_controller.borrow().snapshot());
                }
            }
        });
        
        timer
    };

    let result = window.run();
    controller.borrow_mut().save_session_state(true);
    result
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
    let controller_for_switch = Rc::clone(&controller);
    window.on_switch_to_document(move |index| {
        controller_for_switch.borrow_mut().switch_to_document(index as usize);
        if let Some(window) = weak_window.upgrade() {
            apply_snapshot(&window, &controller_for_switch.borrow().snapshot());
            window.invoke_focus_editor();
        }
    });

    let weak_window = window.as_weak();
    let controller_for_close = Rc::clone(&controller);
    window.on_close_document(move |index| {
        controller_for_close.borrow_mut().close_document(index as usize);
        if let Some(window) = weak_window.upgrade() {
            apply_snapshot(&window, &controller_for_close.borrow().snapshot());
            window.invoke_focus_editor();
        }
    });

    let weak_window = window.as_weak();
    let controller_for_reorder = Rc::clone(&controller);
    window.on_reorder_document(move |from, to| {
        if from < 0 || to < 0 {
            return;
        }
        controller_for_reorder
            .borrow_mut()
            .move_document(from as usize, to as usize);
        if let Some(window) = weak_window.upgrade() {
            apply_snapshot(&window, &controller_for_reorder.borrow().snapshot());
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
    let controller_for_language = Rc::clone(&controller);
    window.on_settings_cycle_language(move |delta| {
        controller_for_language.borrow_mut().cycle_language(delta);
        if let Some(window) = weak_window.upgrade() {
            apply_snapshot(&window, &controller_for_language.borrow().snapshot());
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
            note.has_yank_highlight(ctrl.active_buffer()) || note.has_deferred_action(ctrl.active_buffer())
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
                        {
        let (p, b) = ctrl.active_pane_and_buffer();
        p.clear_yank_highlight(b);
    }
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
        language_label: SharedString::from(snapshot.settings.language_label.as_str()),
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
    window.set_ui_text(UiText {
        app_title: SharedString::from(snapshot.ui_text.app_title.as_str()),
        launcher_hint: SharedString::from(snapshot.ui_text.launcher_hint.as_str()),
        files_menu: SharedString::from(snapshot.ui_text.files_menu.as_str()),
        new_file: SharedString::from(snapshot.ui_text.new_file.as_str()),
        open_file: SharedString::from(snapshot.ui_text.open_file.as_str()),
        save_file: SharedString::from(snapshot.ui_text.save_file.as_str()),
        save_as_file: SharedString::from(snapshot.ui_text.save_as_file.as_str()),
        settings_title: SharedString::from(snapshot.ui_text.settings_title.as_str()),
        settings_close: SharedString::from(snapshot.ui_text.settings_close.as_str()),
        themes_title: SharedString::from(snapshot.ui_text.themes_title.as_str()),
        theme_cancel: SharedString::from(snapshot.ui_text.theme_cancel.as_str()),
        theme_preview: SharedString::from(snapshot.ui_text.theme_preview.as_str()),
        theme_apply: SharedString::from(snapshot.ui_text.theme_apply.as_str()),
        theme_active: SharedString::from(snapshot.ui_text.theme_active.as_str()),
        appearance_section: SharedString::from(snapshot.ui_text.appearance_section.as_str()),
        appearance_themes: SharedString::from(snapshot.ui_text.appearance_themes.as_str()),
        editor_section: SharedString::from(snapshot.ui_text.editor_section.as_str()),
        language: SharedString::from(snapshot.ui_text.language.as_str()),
        font_size: SharedString::from(snapshot.ui_text.font_size.as_str()),
        line_height: SharedString::from(snapshot.ui_text.line_height.as_str()),
        tab_size: SharedString::from(snapshot.ui_text.tab_size.as_str()),
        word_wrap: SharedString::from(snapshot.ui_text.word_wrap.as_str()),
        word_wrap_detail: SharedString::from(snapshot.ui_text.word_wrap_detail.as_str()),
        vim_section: SharedString::from(snapshot.ui_text.vim_section.as_str()),
        sync_clipboard: SharedString::from(snapshot.ui_text.sync_clipboard.as_str()),
        sync_clipboard_detail: SharedString::from(snapshot.ui_text.sync_clipboard_detail.as_str()),
        startup_section: SharedString::from(snapshot.ui_text.startup_section.as_str()),
        restore_last_session: SharedString::from(snapshot.ui_text.restore_last_session.as_str()),
        restore_last_session_detail: SharedString::from(snapshot.ui_text.restore_last_session_detail.as_str()),
        show_launcher_on_startup: SharedString::from(snapshot.ui_text.show_launcher_on_startup.as_str()),
        show_launcher_on_startup_detail: SharedString::from(snapshot.ui_text.show_launcher_on_startup_detail.as_str()),
        window_section: SharedString::from(snapshot.ui_text.window_section.as_str()),
        remember_window_geometry: SharedString::from(snapshot.ui_text.remember_window_geometry.as_str()),
        remember_window_geometry_detail: SharedString::from(snapshot.ui_text.remember_window_geometry_detail.as_str()),
        blur_behind: SharedString::from(snapshot.ui_text.blur_behind.as_str()),
        blur_behind_detail: SharedString::from(snapshot.ui_text.blur_behind_detail.as_str()),
        window_opacity: SharedString::from(snapshot.ui_text.window_opacity.as_str()),
    });
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
    window.set_document_tabs(
        Rc::new(VecModel::from(
            snapshot
                .document_tabs
                .iter()
                .map(|tab| DocumentTab {
                    index: tab.index,
                    title: SharedString::from(tab.title.as_str()),
                    is_active: tab.is_active,
                    is_modified: tab.is_modified,
                })
                .collect::<Vec<_>>(),
        ))
        .into(),
    );
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
    window.invoke_scroll_to_cursor();
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
