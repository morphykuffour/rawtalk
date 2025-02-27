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
sudo $(which nix) run 
# nix run 
```


## Setup for Windows

### Install rustup and cargo

[https://doc.rust-lang.org/cargo/getting-started/installation.html](https://doc.rust-lang.org/cargo/getting-started/installation.html)


### Build and run the project

```powershell
cargo build
cargo run
```

### TODO

- [ ] Add support for Linux and MacOS
- [ ] Make the code keyboard agnostic
