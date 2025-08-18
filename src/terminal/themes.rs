use crate::drivers::framebuffer::RGB;

pub(super) struct Theme {
    pub(super) foreground: RGB,
    pub(super) background: RGB,
    pub(super) selection_foreground: RGB,
    pub(super) selection_background: RGB,
    pub(super) cursor: RGB,
    pub(super) cursor_text_color: RGB,
    pub(super) ansi_colors: [RGB; 16],
}

impl Theme {
    pub(super) const CATPPUCCIN: Theme = Self {
        foreground: RGB::from_hex(0xcdd6f4),
        background: RGB::from_hex(0x1e1e2e),
        selection_foreground: RGB::from_hex(0x1e1e2e),
        selection_background: RGB::from_hex(0xf5e0dc),
        cursor: RGB::from_hex(0xf5e0dc),
        cursor_text_color: RGB::from_hex(0x1e1e2e),
        ansi_colors: [
            RGB::from_hex(0x45475a),
            RGB::from_hex(0xf38ba8),
            RGB::from_hex(0xa6e3a1),
            RGB::from_hex(0xf9e2af),
            RGB::from_hex(0x89b4fa),
            RGB::from_hex(0xf5c2e7),
            RGB::from_hex(0x94e2d5),
            RGB::from_hex(0xbac2de),
            RGB::from_hex(0x585b70),
            RGB::from_hex(0xf38ba8),
            RGB::from_hex(0xa6e3a1),
            RGB::from_hex(0xf9e2af),
            RGB::from_hex(0x89b4fa),
            RGB::from_hex(0xf5c2e7),
            RGB::from_hex(0x94e2d5),
            RGB::from_hex(0xa6adc8),
        ],
    };

    pub(super) const GRUVBOX: Theme = Self {
        foreground: RGB::from_hex(0xebdbb2),
        background: RGB::from_hex(0x282828),
        selection_foreground: RGB::from_hex(0x928374),
        selection_background: RGB::from_hex(0xebdbb2),
        cursor: RGB::from_hex(0x928374),
        cursor_text_color: RGB::from_hex(0x282828),
        ansi_colors: [
            RGB::from_hex(0x665c54),
            RGB::from_hex(0xcc241d),
            RGB::from_hex(0x98971a),
            RGB::from_hex(0xd79921),
            RGB::from_hex(0x458588),
            RGB::from_hex(0xb16286),
            RGB::from_hex(0x689d6a),
            RGB::from_hex(0xa89984),
            RGB::from_hex(0x7c6f64),
            RGB::from_hex(0xfb4934),
            RGB::from_hex(0xb8bb26),
            RGB::from_hex(0xfabd2f),
            RGB::from_hex(0x83a598),
            RGB::from_hex(0xd3869b),
            RGB::from_hex(0x8ec07c),
            RGB::from_hex(0xbdae93),
        ],
    };
}
