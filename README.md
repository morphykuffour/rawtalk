# QMK Layer Switcher (rawtalk)

Cross-platform Raw HID layer switcher for QMK keyboards. Syncs Neovim modes with keyboard layers - use Colemak-DH in insert mode and QWERTY in normal mode.

## How It Works

1. Neovim writes the current mode to a file (`/tmp/vim_mode` on Unix, `C:\tmp\vim_mode.txt` on Windows)
2. This tool monitors the file and sends Raw HID commands to your QMK keyboard
3. The keyboard switches layers based on the mode:
   - Insert mode (`i`) → Layer 0 (Colemak-DH)
   - Other modes → Layer 3 (QWERTY)

## Platform Support

| Platform | Status | Notes |
|----------|--------|-------|
| Windows  | ✅ Works | No special setup needed |
| Linux    | ✅ Works | Requires udev rules |
| macOS    | ✅ Works | May need Input Monitoring permission |

## Setup

### Prerequisites

- Rust toolchain: https://rustup.rs/
- A QMK keyboard with Raw HID enabled (see QMK Config below)

### Linux Setup

**1. Install udev rules** (required for non-root HID access):

```bash
sudo tee /etc/udev/rules.d/70-qmk-keyboard.rules << 'RULES'
# QMK Ferris Sweep - Raw HID access
SUBSYSTEMS=="usb", ATTRS{idVendor}=="c2ab", ATTRS{idProduct}=="3939", TAG+="uaccess"
KERNEL=="hidraw*", ATTRS{idVendor}=="c2ab", ATTRS{idProduct}=="3939", TAG+="uaccess"
RULES

# Reload rules
sudo udevadm control --reload-rules
sudo udevadm trigger
```

**2. Reconnect your keyboard** after installing rules.

### macOS Setup

If you get permission errors, grant Input Monitoring access:
1. Open System Preferences → Security & Privacy → Privacy
2. Select "Input Monitoring" from the left sidebar
3. Add your terminal app or the compiled binary

### Windows Setup

No special setup required. Windows handles HID access automatically.

### Build and Run

```bash
# Build
cargo build --release

# Run (set VIM_MODE_FILE env var to customize path)
cargo run --release

# Or with Nix
nix develop
nix run
```

### Environment Variables

- `VIM_MODE_FILE`: Path to the vim mode file (default: `/tmp/vim_mode` on Unix, `C:\tmp\vim_mode.txt` on Windows)

## Neovim Configuration

Add this to your Neovim config to write the mode to a file:

```lua
-- Write vim mode to file for keyboard layer switching
local mode_file = vim.fn.has('win32') == 1 and 'C:\\tmp\\vim_mode.txt' or '/tmp/vim_mode'

vim.api.nvim_create_autocmd('ModeChanged', {
  pattern = '*',
  callback = function()
    local mode = vim.fn.mode()
    local file = io.open(mode_file, 'w')
    if file then
      file:write(mode)
      file:close()
    end
  end,
})

-- Write initial mode on startup
vim.api.nvim_create_autocmd('VimEnter', {
  callback = function()
    local mode = vim.fn.mode()
    local file = io.open(mode_file, 'w')
    if file then
      file:write(mode)
      file:close()
    end
  end,
})
```

## QMK Keyboard Configuration

Add this to your `keymap.c`:

```c
#include <raw_hid.h>

bool raw_hid_receive_kb(uint8_t *data, uint8_t length) {
    switch(data[0]) {  // Command byte
        case 0x40:  // Get current layer
            data[0] = (uint8_t)get_highest_layer(layer_state);
            raw_hid_send(data, length);
            return true;

        case 0x00: {  // Layer switch command
            uint8_t target_layer = data[1];
            if (target_layer <= 3) {
                layer_move(target_layer);
                data[0] = 0x00;        // Success
                data[1] = get_highest_layer(layer_state);
                data[2] = 0xAA;        // Acknowledgment
            } else {
                data[0] = 0xFF;  // Error
            }
            raw_hid_send(data, length);
            return true;
        }
    }
    return false;
}
```

And in your `rules.mk`:

```makefile
RAW_ENABLE = yes
```

## Customization

### Changing VID/PID

Edit `src/main.rs` and change these constants:

```rust
const KEYBOARD_VID: u16 = 0xC2AB;  // Your keyboard's Vendor ID
const KEYBOARD_PID: u16 = 0x3939;  // Your keyboard's Product ID
```

### Changing Layer Mapping

Edit the layer assignments in `src/main.rs`:

```rust
let layer = if current_mode == "i" {
    0u8  // Insert mode layer (Colemak-DH)
} else {
    3u8  // Normal mode layer (QWERTY)
};
```

## Troubleshooting

### "Keyboard not found or access denied"

- **Linux**: Make sure udev rules are installed and you've reconnected the keyboard
- **macOS**: Check Input Monitoring permissions
- **All platforms**: Verify VID/PID match your keyboard (use `lsusb` on Linux, System Information on macOS)

### No response from keyboard

- Ensure `RAW_ENABLE = yes` is in your `rules.mk`
- Check that `raw_hid_receive_kb` is implemented in your keymap
- Enable console debugging: `CONSOLE_ENABLE = yes` in `rules.mk`

## License

MIT
