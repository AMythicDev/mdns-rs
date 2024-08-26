FROM rust:slim
WORKDIR /mdns-rs
COPY Cargo.toml ./
COPY src/lib.rs ./src/
RUN cargo fetch
COPY . ./
