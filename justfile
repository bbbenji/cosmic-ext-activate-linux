# Settings app binary (also the package name).
name := 'cosmic-ext-activate-linux'
# Watermark daemon binary.
daemon := 'cosmic-ext-activate-linux-daemon'
# The unique ID of the application.
appid := 'io.github.bbbenji.ActivateLinux'

# Path to root file system, which defaults to `/`.
rootdir := ''
# The prefix for the `/usr` directory.
prefix := '/usr'
# The location of the cargo target directory.
cargo-target-dir := env('CARGO_TARGET_DIR', 'target')

base-dir := absolute_path(clean(rootdir / prefix))
etc-dir := absolute_path(clean(rootdir / '/etc'))

bin-dst := base-dir / 'bin' / name
daemon-dst := base-dir / 'bin' / daemon
desktop-dst := base-dir / 'share' / 'applications' / (appid + '.desktop')
metainfo-dst := base-dir / 'share' / 'metainfo' / (appid + '.metainfo.xml')
icons-dir := base-dir / 'share' / 'icons' / 'hicolor'
# Raster icon sizes shipped under resources/icons/hicolor.
icon-sizes := '32 48 64 128 256'
# Placeholder SVG installed by early builds; removed on install so it can't shadow the PNGs.
old-icon-dst := icons-dir / 'scalable' / 'apps' / (appid + '.svg')
autostart-dst := etc-dir / 'xdg' / 'autostart' / (appid + '.Daemon.desktop')

# Default recipe which runs `just build-release`
default: build-release

# Runs `cargo clean`
clean:
    cargo clean

# Removes vendored dependencies
clean-vendor:
    rm -rf .cargo vendor vendor.tar

# `cargo clean` and removes vendored dependencies
clean-dist: clean clean-vendor

# Compiles with debug profile
build-debug *args:
    cargo build --workspace --locked {{args}}

# Compiles with release profile
build-release *args: (build-debug '--release' args)

# Compiles release profile with vendored dependencies
build-vendored *args: vendor-extract (build-release '--frozen --offline' args)

# Runs a clippy check
check *args:
    cargo clippy --workspace --all-targets --locked {{args}} -- -W clippy::pedantic

# Runs a clippy check with JSON message format
check-json: (check '--message-format=json')

# Runs the unit tests
test *args:
    cargo test --workspace --locked {{args}}

# Runs the watermark daemon for testing
run-daemon *args:
    env RUST_BACKTRACE=full RUST_LOG=${RUST_LOG:-info} cargo run --release --locked -p {{daemon}} -- {{args}}

# Runs the settings app for testing
run-settings *args:
    env RUST_BACKTRACE=full cargo run --release --locked -p {{name}} -- {{args}}

alias run := run-settings

# Installs files
install:
    install -Dm0755 {{ cargo-target-dir / 'release' / name }} {{bin-dst}}
    install -Dm0755 {{ cargo-target-dir / 'release' / daemon }} {{daemon-dst}}
    install -Dm0644 {{ 'target' / 'xdgen' / (appid + '.desktop') }} {{desktop-dst}}
    install -Dm0644 {{ 'target' / 'xdgen' / (appid + '.metainfo.xml') }} {{metainfo-dst}}
    for size in {{icon-sizes}}; do install -Dm0644 resources/icons/hicolor/${size}x${size}/apps/{{appid}}.png {{icons-dir}}/${size}x${size}/apps/{{appid}}.png; done
    rm -f {{old-icon-dst}}
    install -Dm0644 {{ 'resources' / (appid + '.Daemon.desktop') }} {{autostart-dst}}
    @echo 'Installed. Start the watermark with: cosmic-ext-activate-linux-daemon &'
    @echo '(A running systemd --user manager only picks up the new autostart entry after'
    @echo ' "systemctl --user daemon-reload" or a reboot; logging out may not be enough.)'

# Uninstalls installed files
uninstall:
    rm -f {{bin-dst}} {{daemon-dst}} {{desktop-dst}} {{metainfo-dst}} {{autostart-dst}} {{old-icon-dst}}
    for size in {{icon-sizes}}; do rm -f {{icons-dir}}/${size}x${size}/apps/{{appid}}.png; done

# Vendor dependencies locally
vendor:
    mkdir -p .cargo
    cargo vendor | head -n -1 > .cargo/config.toml
    echo 'directory = "vendor"' >> .cargo/config.toml
    tar pcf vendor.tar vendor
    rm -rf vendor

# Extracts vendored dependencies
vendor-extract:
    rm -rf vendor
    tar pxf vendor.tar

# Bump cargo version, create git commit, and create tag
tag version:
    sed -i '0,/^version/s/^version.*/version = "{{version}}"/' Cargo.toml
    cargo check --workspace
    git add Cargo.toml Cargo.lock
    git commit -m 'release: {{version}}'
    git tag -a {{version}} -m ''
