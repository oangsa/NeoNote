use crate::rpc::{api, RpcClient};

use super::Theme;

pub fn apply_theme_to_neovim(client: &RpcClient, theme: &Theme) {
    api::set_colorscheme(client, &theme.neovim_colorscheme);
    api::set_neovide_cursor_globals(
        client,
        theme.neovide.cursor_animation_length,
        theme.neovide.cursor_trail_size,
        &theme.neovide.cursor_vfx_mode,
    );
}
