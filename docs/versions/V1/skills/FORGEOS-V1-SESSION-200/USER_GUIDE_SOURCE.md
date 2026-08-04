# FORGEOS-V1-SESSION-200 User Guide Source

## What this capability provides

ForgeOS includes a dedicated session launcher and a canonical display-manager entry
source. The launcher starts the installed ForgeOS composition root directly rather
than passing through a user shell or a repository checkout.

Canonical installed paths declared by this capability:

```text
/usr/libexec/forgeos/forgeos-session-launcher
/usr/bin/forge-app
```

The desktop-entry source is:

```text
crates/forge-session/assets/forgeos.desktop
```

A later packaging capability installs that source into the display manager's
supported session directory. This capability deliberately does not modify the live
host during tests.

## Development invocation

After building the binaries, the launcher can target a development composition root
explicitly:

```bash
cargo build --locked -p forge-app -p forge-session --bin forgeos-session-launcher

target/debug/forgeos-session-launcher \
  --composition-root "$PWD/target/debug/forge-app"
```

The launcher requires a real display-manager-style environment with an absolute
`HOME`, an absolute `XDG_RUNTIME_DIR`, and either `WAYLAND_DISPLAY` or `DISPLAY`.
It exits with the composition root's actual normal exit code.

## Failure witness

A missing composition root returns a real nonzero startup result instead of a fake
successful session:

```bash
target/debug/forgeos-session-launcher \
  --composition-root /definitely/missing/forge-app

echo "$?"  # 127
```

## Safety cutline

The launcher clears the ambient process environment and restores only the declared
session variables. It never sources `.profile`, `.bashrc`, or another shell hook,
and it always starts the composition root from `/` rather than the current terminal
or repository directory.
