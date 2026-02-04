default: server

install:
    cargo install --path .

server:
    cargo run --bin epu-server

linux-build:
    cargo zigbuild --target x86_64-unknown-linux-musl --release