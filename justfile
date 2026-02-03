default: server

install:
    cargo install --path .

server:
    cargo run --bin epu-server
