use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct StartupArgs {
    pub files_to_open: Vec<PathBuf>,
    pub register_file_associations: bool,
    pub unregister_file_associations: bool,
}

pub fn parse_startup_args() -> StartupArgs {
    let mut args = StartupArgs::default();

    for arg in std::env::args_os().skip(1) {
        let arg_string = arg.to_string_lossy();

        match arg_string.as_ref() {
            "--register-file-associations" => {
                args.register_file_associations = true;
            }
            "--unregister-file-associations" => {
                args.unregister_file_associations = true;
            }
            _ => {
                args.files_to_open.push(PathBuf::from(arg));
            }
        }
    }

    args
}
