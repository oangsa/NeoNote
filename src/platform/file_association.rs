#[cfg(target_os = "windows")]
use windows::Win32::UI::Shell::{SHChangeNotify, SHCNE_ASSOCCHANGED, SHCNF_IDLIST};

#[derive(Debug, Clone)]
pub struct FileAssociationConfig {
    pub app_name: String,
    pub app_exe_name: String,
    pub app_path: std::path::PathBuf,
    pub prog_id: String,
    pub friendly_file_type_name: String,
    pub extensions: Vec<String>,
}

#[cfg(target_os = "windows")]
pub fn register_file_associations(config: &FileAssociationConfig) -> anyhow::Result<()> {
    use winreg::{enums::*, RegKey};

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let classes = hkcu.create_subkey("Software\\Classes")?.0;

    // 1. Register the application
    let app_key_path = format!("Applications\\{}", config.app_exe_name);
    let (app_key, _) = classes.create_subkey(&app_key_path)?;
    app_key.set_value("FriendlyAppName", &config.app_name)?;
    app_key.set_value("SupportedTypes", &"")?; // Base key, populate later if needed

    for ext in &config.extensions {
        let (supported_types, _) = app_key.create_subkey("SupportedTypes")?;
        supported_types.set_value(ext, &"")?;
    }

    let command_path = format!("{}\\shell\\open\\command", app_key_path);
    let (cmd_key, _) = classes.create_subkey(&command_path)?;
    let exe_path_str = config.app_path.to_string_lossy();
    let command_string = format!("\"{}\" \"%1\"", exe_path_str);
    cmd_key.set_value("", &command_string)?;

    // 2. Register the ProgID
    let (prog_id_key, _) = classes.create_subkey(&config.prog_id)?;
    prog_id_key.set_value("", &config.friendly_file_type_name)?;

    let (icon_key, _) = prog_id_key.create_subkey("DefaultIcon")?;
    let icon_string = format!("\"{}\",0", exe_path_str);
    icon_key.set_value("", &icon_string)?;

    let (prog_cmd_key, _) = prog_id_key.create_subkey("shell\\open\\command")?;
    prog_cmd_key.set_value("", &command_string)?;

    // 3. Register extensions with OpenWithProgids
    for ext in &config.extensions {
        let (ext_key, _) = classes.create_subkey(ext)?;
        let (open_with_progids, _) = ext_key.create_subkey("OpenWithProgids")?;
        open_with_progids.set_value(&config.prog_id, &"")?;
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn register_file_associations(_config: &FileAssociationConfig) -> anyhow::Result<()> {
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn unregister_file_associations(config: &FileAssociationConfig) -> anyhow::Result<()> {
    use winreg::{enums::*, RegKey};

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(classes) = hkcu.open_subkey_with_flags("Software\\Classes", KEY_ALL_ACCESS) {
        let app_key_path = format!("Applications\\{}", config.app_exe_name);
        let _ = classes.delete_subkey_all(&app_key_path);
        let _ = classes.delete_subkey_all(&config.prog_id);

        for ext in &config.extensions {
            if let Ok(ext_key) = classes.open_subkey_with_flags(ext, KEY_ALL_ACCESS) {
                if let Ok(open_with_progids) = ext_key.open_subkey_with_flags("OpenWithProgids", KEY_ALL_ACCESS) {
                    let _ = open_with_progids.delete_value(&config.prog_id);
                }
            }
        }
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn unregister_file_associations(_config: &FileAssociationConfig) -> anyhow::Result<()> {
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn is_registered(config: &FileAssociationConfig) -> anyhow::Result<bool> {
    use winreg::{enums::*, RegKey};

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let classes = match hkcu.open_subkey("Software\\Classes") {
        Ok(k) => k,
        Err(_) => return Ok(false),
    };

    let app_key_path = format!("Applications\\{}", config.app_exe_name);
    if classes.open_subkey(&app_key_path).is_err() {
        return Ok(false);
    }
    
    if classes.open_subkey(&config.prog_id).is_err() {
        return Ok(false);
    }

    Ok(true)
}

#[cfg(not(target_os = "windows"))]
pub fn is_registered(_config: &FileAssociationConfig) -> anyhow::Result<bool> {
    Ok(false)
}

#[cfg(target_os = "windows")]
pub fn notify_shell_association_changed() {
    unsafe {
        SHChangeNotify(
            SHCNE_ASSOCCHANGED,
            SHCNF_IDLIST,
            None,
            None,
        );
    }
}

#[cfg(not(target_os = "windows"))]
pub fn notify_shell_association_changed() {}
