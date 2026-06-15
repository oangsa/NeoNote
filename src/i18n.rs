use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AppLanguage {
    #[default]
    English,
    Spanish,
    Japanese,
}

impl AppLanguage {
    pub fn cycle(self, delta: i32) -> Self {
        const ORDER: [AppLanguage; 3] = [
            AppLanguage::English,
            AppLanguage::Spanish,
            AppLanguage::Japanese,
        ];

        let current = ORDER.iter().position(|language| *language == self).unwrap_or(0) as i32;
        let next = (current + delta).rem_euclid(ORDER.len() as i32) as usize;
        ORDER[next]
    }

    pub fn native_name(self) -> &'static str {
        match self {
            AppLanguage::English => "English",
            AppLanguage::Spanish => "Español",
            AppLanguage::Japanese => "日本語",
        }
    }

    pub fn strings(self) -> UiTextSnapshot {
        match self {
            AppLanguage::English => UiTextSnapshot {
                app_title: "NeoNote".to_string(),
                launcher_hint: "New File\nOpen File".to_string(),
                files_menu: "Files".to_string(),
                new_file: "New".to_string(),
                open_file: "Open...".to_string(),
                save_file: "Save".to_string(),
                save_as_file: "Save As...".to_string(),
                settings_title: "Settings".to_string(),
                settings_close: "Close".to_string(),
                themes_title: "Themes".to_string(),
                theme_cancel: "Cancel".to_string(),
                theme_preview: "Preview".to_string(),
                theme_apply: "Apply".to_string(),
                theme_active: "Active".to_string(),
                appearance_section: "Appearance".to_string(),
                appearance_themes: "Themes".to_string(),
                editor_section: "Editor".to_string(),
                language: "Language".to_string(),
                font_size: "Font size".to_string(),
                line_height: "Line height".to_string(),
                tab_size: "Tab size".to_string(),
                word_wrap: "Word wrap".to_string(),
                word_wrap_detail: "Soft-wrap long note lines".to_string(),
                vim_section: "Vim Mode".to_string(),
                sync_clipboard: "Sync with clipboard".to_string(),
                sync_clipboard_detail: "Use the system clipboard for yanks and pastes".to_string(),
                startup_section: "Startup".to_string(),
                restore_last_session: "Restore last session".to_string(),
                restore_last_session_detail: "Open remembered notes on launch".to_string(),
                show_launcher_on_startup: "Show launcher on startup".to_string(),
                show_launcher_on_startup_detail: "Start on the note launcher when no session opens".to_string(),
                window_section: "Window".to_string(),
                remember_window_geometry: "Remember window geometry".to_string(),
                remember_window_geometry_detail: "Persist size and placement when supported".to_string(),
                blur_behind: "Blur behind".to_string(),
                blur_behind_detail: "Use the Windows blur effect when supported".to_string(),
                window_opacity: "Window opacity".to_string(),
            },
            AppLanguage::Spanish => UiTextSnapshot {
                app_title: "NeoNote".to_string(),
                launcher_hint: "Nuevo archivo\nAbrir archivo".to_string(),
                files_menu: "Archivos".to_string(),
                new_file: "Nuevo".to_string(),
                open_file: "Abrir...".to_string(),
                save_file: "Guardar".to_string(),
                save_as_file: "Guardar como...".to_string(),
                settings_title: "Configuración".to_string(),
                settings_close: "Cerrar".to_string(),
                themes_title: "Temas".to_string(),
                theme_cancel: "Cancelar".to_string(),
                theme_preview: "Vista previa".to_string(),
                theme_apply: "Aplicar".to_string(),
                theme_active: "Activo".to_string(),
                appearance_section: "Apariencia".to_string(),
                appearance_themes: "Temas".to_string(),
                editor_section: "Editor".to_string(),
                language: "Idioma".to_string(),
                font_size: "Tamaño de fuente".to_string(),
                line_height: "Altura de línea".to_string(),
                tab_size: "Tamaño de tabulación".to_string(),
                word_wrap: "Ajuste de línea".to_string(),
                word_wrap_detail: "Ajustar visualmente las líneas largas".to_string(),
                vim_section: "Modo Vim".to_string(),
                sync_clipboard: "Sincronizar con el portapapeles".to_string(),
                sync_clipboard_detail: "Usar el portapapeles del sistema para copiar y pegar".to_string(),
                startup_section: "Inicio".to_string(),
                restore_last_session: "Restaurar la última sesión".to_string(),
                restore_last_session_detail: "Abrir las notas recordadas al iniciar".to_string(),
                show_launcher_on_startup: "Mostrar el lanzador al iniciar".to_string(),
                show_launcher_on_startup_detail: "Abrir el lanzador si no hay sesión para restaurar".to_string(),
                window_section: "Ventana".to_string(),
                remember_window_geometry: "Recordar geometría de la ventana".to_string(),
                remember_window_geometry_detail: "Guardar tamaño y posición cuando sea posible".to_string(),
                blur_behind: "Desenfoque detrás".to_string(),
                blur_behind_detail: "Usar el efecto de desenfoque de Windows cuando esté disponible".to_string(),
                window_opacity: "Opacidad de la ventana".to_string(),
            },
            AppLanguage::Japanese => UiTextSnapshot {
                app_title: "NeoNote".to_string(),
                launcher_hint: "新規ファイル\nファイルを開く".to_string(),
                files_menu: "ファイル".to_string(),
                new_file: "新規".to_string(),
                open_file: "開く...".to_string(),
                save_file: "保存".to_string(),
                save_as_file: "名前を付けて保存...".to_string(),
                settings_title: "設定".to_string(),
                settings_close: "閉じる".to_string(),
                themes_title: "テーマ".to_string(),
                theme_cancel: "キャンセル".to_string(),
                theme_preview: "プレビュー".to_string(),
                theme_apply: "適用".to_string(),
                theme_active: "有効".to_string(),
                appearance_section: "表示".to_string(),
                appearance_themes: "テーマ".to_string(),
                editor_section: "エディター".to_string(),
                language: "言語".to_string(),
                font_size: "フォントサイズ".to_string(),
                line_height: "行の高さ".to_string(),
                tab_size: "タブ幅".to_string(),
                word_wrap: "折り返し".to_string(),
                word_wrap_detail: "長い行を画面上で折り返す".to_string(),
                vim_section: "Vim モード".to_string(),
                sync_clipboard: "クリップボードと同期".to_string(),
                sync_clipboard_detail: "ヤンクと貼り付けにシステムのクリップボードを使う".to_string(),
                startup_section: "起動".to_string(),
                restore_last_session: "前回のセッションを復元".to_string(),
                restore_last_session_detail: "起動時に前回のノートを開く".to_string(),
                show_launcher_on_startup: "起動時にランチャーを表示".to_string(),
                show_launcher_on_startup_detail: "復元するセッションがない場合はランチャーを開く".to_string(),
                window_section: "ウィンドウ".to_string(),
                remember_window_geometry: "ウィンドウ位置を記憶".to_string(),
                remember_window_geometry_detail: "対応時はサイズと位置を保存する".to_string(),
                blur_behind: "背景のぼかし".to_string(),
                blur_behind_detail: "対応時は Windows のぼかし効果を使う".to_string(),
                window_opacity: "ウィンドウの不透明度".to_string(),
            },
        }
    }

    pub fn untitled_label(self) -> &'static str {
        match self {
            AppLanguage::English => "Untitled",
            AppLanguage::Spanish => "Sin título",
            AppLanguage::Japanese => "無題",
        }
    }

    pub fn ready_mode_label(self) -> &'static str {
        match self {
            AppLanguage::English => "READY",
            AppLanguage::Spanish => "LISTO",
            AppLanguage::Japanese => "準備完了",
        }
    }

    pub fn no_name_status(self) -> &'static str {
        match self {
            AppLanguage::English => " [No Name]",
            AppLanguage::Spanish => " [Sin nombre]",
            AppLanguage::Japanese => " [名前なし]",
        }
    }

    pub fn substitute_confirm(self, replacement: &str) -> String {
        match self {
            AppLanguage::English => format!("replace with {replacement} (y/n/a/q/l)?"),
            AppLanguage::Spanish => format!("reemplazar con {replacement} (y/n/a/q/l)?"),
            AppLanguage::Japanese => format!("{replacement} に置換しますか (y/n/a/q/l)?"),
        }
    }

    pub fn status_right(
        self,
        document_index: usize,
        document_count: usize,
        line: usize,
        column: usize,
        search: Option<&str>,
        line_count: usize,
        word_count: usize,
        char_count: usize,
    ) -> String {
        let search_suffix = search
            .map(|pattern| format!("  |  /{pattern}"))
            .unwrap_or_default();

        match self {
            AppLanguage::English => format!(
                "Doc {document_index}/{document_count}  |  Ln {line}, Col {column}{search_suffix}  |  {line_count} lines, {word_count} words, {char_count} chars  "
            ),
            AppLanguage::Spanish => format!(
                "Doc {document_index}/{document_count}  |  Lín {line}, Col {column}{search_suffix}  |  {line_count} líneas, {word_count} palabras, {char_count} caracteres  "
            ),
            AppLanguage::Japanese => format!(
                "文書 {document_index}/{document_count}  |  行 {line}, 列 {column}{search_suffix}  |  {line_count} 行, {word_count} 語, {char_count} 文字  "
            ),
        }
    }

    pub fn ready_status_right(self) -> &'static str {
        match self {
            AppLanguage::English => "NeoNote native Vim core  ",
            AppLanguage::Spanish => "Núcleo Vim nativo de NeoNote  ",
            AppLanguage::Japanese => "NeoNote ネイティブ Vim コア  ",
        }
    }

    pub fn new_note_ready_message(self) -> &'static str {
        match self {
            AppLanguage::English => "New note ready.",
            AppLanguage::Spanish => "Nueva nota lista.",
            AppLanguage::Japanese => "新しいノートを用意しました。",
        }
    }

    pub fn saved_message(self) -> &'static str {
        match self {
            AppLanguage::English => "Saved.",
            AppLanguage::Spanish => "Guardado.",
            AppLanguage::Japanese => "保存しました。",
        }
    }

    pub fn choose_theme_message(self) -> &'static str {
        match self {
            AppLanguage::English => "Choose a theme.",
            AppLanguage::Spanish => "Elige un tema.",
            AppLanguage::Japanese => "テーマを選択してください。",
        }
    }

    pub fn previewing_theme_message(self, theme_name: &str) -> String {
        match self {
            AppLanguage::English => format!("Previewing {theme_name}."),
            AppLanguage::Spanish => format!("Vista previa de {theme_name}."),
            AppLanguage::Japanese => format!("{theme_name} をプレビュー中です。"),
        }
    }

    pub fn theme_applied_message(self) -> &'static str {
        match self {
            AppLanguage::English => "Theme applied.",
            AppLanguage::Spanish => "Tema aplicado.",
            AppLanguage::Japanese => "テーマを適用しました。",
        }
    }

    pub fn theme_selection_canceled_message(self) -> &'static str {
        match self {
            AppLanguage::English => "Theme selection canceled.",
            AppLanguage::Spanish => "Selección de tema cancelada.",
            AppLanguage::Japanese => "テーマ選択をキャンセルしました。",
        }
    }

    pub fn adjust_settings_message(self) -> &'static str {
        match self {
            AppLanguage::English => "Adjust settings.",
            AppLanguage::Spanish => "Ajusta la configuración.",
            AppLanguage::Japanese => "設定を調整してください。",
        }
    }

    pub fn settings_closed_message(self) -> &'static str {
        match self {
            AppLanguage::English => "Settings closed.",
            AppLanguage::Spanish => "Configuración cerrada.",
            AppLanguage::Japanese => "設定を閉じました。",
        }
    }

    pub fn switched_to_message(self, title: &str) -> String {
        match self {
            AppLanguage::English => format!("Switched to {title}."),
            AppLanguage::Spanish => format!("Cambiado a {title}."),
            AppLanguage::Japanese => format!("{title} に切り替えました。"),
        }
    }

    pub fn closed_note_message(self) -> &'static str {
        match self {
            AppLanguage::English => "Closed note.",
            AppLanguage::Spanish => "Nota cerrada.",
            AppLanguage::Japanese => "ノートを閉じました。",
        }
    }

    pub fn ready_message(self) -> &'static str {
        match self {
            AppLanguage::English => "Ready.",
            AppLanguage::Spanish => "Listo.",
            AppLanguage::Japanese => "準備完了。",
        }
    }

    pub fn file_already_open_message(self) -> &'static str {
        match self {
            AppLanguage::English => "File already open.",
            AppLanguage::Spanish => "El archivo ya está abierto.",
            AppLanguage::Japanese => "ファイルはすでに開かれています。",
        }
    }

    pub fn opened_note_message(self) -> &'static str {
        match self {
            AppLanguage::English => "Opened note.",
            AppLanguage::Spanish => "Nota abierta.",
            AppLanguage::Japanese => "ノートを開きました。",
        }
    }

    pub fn no_write_since_last_change(self) -> &'static str {
        match self {
            AppLanguage::English => "No write since last change (add ! to override)",
            AppLanguage::Spanish => "No se ha guardado desde el último cambio (añade ! para forzar)",
            AppLanguage::Japanese => "最後の変更以降に保存されていません (! で強制します)",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UiTextSnapshot {
    pub app_title: String,
    pub launcher_hint: String,
    pub files_menu: String,
    pub new_file: String,
    pub open_file: String,
    pub save_file: String,
    pub save_as_file: String,
    pub settings_title: String,
    pub settings_close: String,
    pub themes_title: String,
    pub theme_cancel: String,
    pub theme_preview: String,
    pub theme_apply: String,
    pub theme_active: String,
    pub appearance_section: String,
    pub appearance_themes: String,
    pub editor_section: String,
    pub language: String,
    pub font_size: String,
    pub line_height: String,
    pub tab_size: String,
    pub word_wrap: String,
    pub word_wrap_detail: String,
    pub vim_section: String,
    pub sync_clipboard: String,
    pub sync_clipboard_detail: String,
    pub startup_section: String,
    pub restore_last_session: String,
    pub restore_last_session_detail: String,
    pub show_launcher_on_startup: String,
    pub show_launcher_on_startup_detail: String,
    pub window_section: String,
    pub remember_window_geometry: String,
    pub remember_window_geometry_detail: String,
    pub blur_behind: String,
    pub blur_behind_detail: String,
    pub window_opacity: String,
}
