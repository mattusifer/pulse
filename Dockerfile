FROM rust:1.35-stretch

RUN cargo install diesel_cli

COPY ./Cargo.toml ./Cargo.lock ./diesel.toml /src/
COPY ./src /src/src
COPY ./resources /src/resources
COPY ./migrations /src/migrations

RUN cargo install --path /src

ENTRYPOINT diesel migration --migration-dir /src/migrations run && pulse
