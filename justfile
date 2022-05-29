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
    watch . { cargo tests } --glob=**/*.rs

build-release:
    cargo build --release
    @$"Build size: (ls target/release/templater | get size)"

publish-to-local-bin: build-release
    cp target/release/templater ~/bin/
