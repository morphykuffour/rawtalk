## Install the nix package manager

```bash
curl --proto '=https' --tlsv1.2 -sSf -L https://install.determinate.systems/nix | sh -s -- install
```

## Build and run the project

```bash
nix develop -c $SHELL
nix build
sudo $(which nix) run 
# nix run 
```


