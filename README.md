> Note: That this project works only in windows 11. Linux and MacOS are are still under development.

## Setup for Linux and MacoS

### Install the nix package manager

```bash
curl --proto '=https' --tlsv1.2 -sSf -L https://install.determinate.systems/nix | sh -s -- install
```

### Build and run the project

```bash
nix develop -c $SHELL
nix build
nix run 
# sudo $(which nix) run 
```


## Setup for Windows

### Install rustup and cargo

[https://doc.rust-lang.org/cargo/getting-started/installation.html](https://doc.rust-lang.org/cargo/getting-started/installation.html)


### Build and run the project

```powershell
cargo build
cargo run
```

### QMK Config

```c
void raw_hid_receive(uint8_t *data, uint8_t length) {
    dprintf("\nReceived raw HID packet (length=%d):\n", length);
    // Print every byte received
    for (uint8_t i = 0; i < length; i++) {
        dprintf("data[%d] = 0x%02X\n", i, data[i]);
    }

    dprintf("Current layer state: 0x%08X\n", (unsigned int)layer_state);
    dprintf("Highest active layer: %d\n", get_highest_layer(layer_state));

    uint8_t command = data[0];  // Command is in first byte
    uint8_t layer = data[1];    // Layer is in second byte

    switch(command) {
        case 0x03: {  // Layer switch command
            dprintf("Command: Layer switch (0x00) to layer %d\n", layer);

            if (layer <= 3) {
                layer_clear();  // Clear all layers first
                layer_move(0);  // Force the layer to 0 (colemak-dh)

                uint8_t current = get_highest_layer(layer_state);
                dprintf("New layer: %d\n", current);

                // Must clear data buffer before setting response
                memset(data, 0, length);
                data[0] = 0x00;        // Success
                data[1] = current;      // Current layer
                data[2] = 0xAA;        // Acknowledgment

                dprintf("Layer switch successful\n");
            } else {
                dprintf("Invalid layer %d requested\n", layer);
                memset(data, 0, length);
                data[0] = 0xFF;  // Error
            }

            dprintf("Sending response\n");
            raw_hid_send(data, length);
            break;
        }

        default: {
            dprintf("Switching to qwerty\n");

            layer_clear();  // Clear all layers first
            layer_move(3);  // Force the layer to 3 (qwerty)

            uint8_t current = get_highest_layer(layer_state);
            dprintf("New layer: %d\n", current);

            memset(data, 0, length);
            data[0] = 0xFF;
            raw_hid_send(data, length);
            break;
        }
    }
}
```
### TODO

- [ ] Add support for Linux and MacOS
- [ ] Make the code keyboard agnostic
