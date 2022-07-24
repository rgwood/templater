set shell := ["nu", "-c"]

watch:
    watch . { cargo run } --glob=**/*.rs

watch-scratch:
    watch . { cargo run --bin scratch } --glob=**/*.rs

# use watchexec because Nu watch can't kill process to restart it
watch-serve:
    watchexec --exts=rs --on-busy-update=restart -- cargo run -- --serve

run:
    cargo run

test:
    cargo test

watch-tests:
    watch . { cargo test } --glob=**/*.rs

expected_filename := if os_family() == "windows" { "templater.exe" } else { "templater" }

build-release:
    cargo build --release
    ls target/release

publish-to-local-bin: build-release
    cp target/release/{{ expected_filename }} ~/bin/

build-all: build-linux-x64 build-linux-arm64

build-linux-x64:
    cross build --target x86_64-unknown-linux-gnu --release

build-linux-arm64:
    cross build --target aarch64-unknown-linux-gnu --release

build-windows-on-linux:
    cross build --target x86_64-pc-windows-gnu --release
