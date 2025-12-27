# rawtalk

Sync Neovim mode with QMK keyboard layers via Raw HID.

- Insert mode -> Colemak-DH (layer 0)
- Normal/other modes -> QWERTY (layer 3)

## Setup

### 1. Build & Run
```bash
cargo build --release
./target/release/rawtalk
```

### 2. Neovim Config
Add to `init.lua`:
```lua
local socket, client, connected = "/tmp/rawtalk.sock", nil, false
local uv = vim.loop

local function send(mode)
    if not connected then
        client = uv.new_pipe()
        client:connect(socket, function(err) connected = not err end)
        vim.wait(50, function() return connected end)
    end
    if connected then pcall(function() client:write(mode .. "\n") end) end
end

vim.api.nvim_create_autocmd("ModeChanged", {
    callback = function() send(vim.fn.mode()) end
})
```

### 3. QMK Keymap
Add to `keymap.c`:
```c
void raw_hid_receive_kb(uint8_t *data, uint8_t length) {
    if (data[0] == 0x00 && data[1] <= 3) {
        set_single_default_layer(data[1]);
        data[0] = 0x00;
        data[1] = data[1];
        data[2] = 0xAA;
        raw_hid_send(data, length);
    }
}
```

Enable in `rules.mk`:
```makefile
RAW_ENABLE = yes
```

### Linux udev rules
```bash
echo 'SUBSYSTEMS=="usb", ATTRS{idVendor}=="c2ab", ATTRS{idProduct}=="3939", TAG+="uaccess"' | \
  sudo tee /etc/udev/rules.d/70-qmk.rules
sudo udevadm control --reload-rules
```

## Config

| Env Variable | Default | Description |
|--------------|---------|-------------|
| `RAWTALK_SOCKET` | `/tmp/rawtalk.sock` | Socket path |

Edit `VID`/`PID` in `main.rs` for different keyboards.
