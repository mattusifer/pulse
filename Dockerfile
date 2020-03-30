###############################################################################
# TARGET: build
###############################################################################

FROM rust:1.42-slim as build

RUN apt-get -y update && apt-get -y install libpq-dev pkg-config nodejs npm

# dependencies
RUN npm install -g npm@latest
RUN npm install -g @angular/cli
RUN cargo install diesel_cli --no-default-features --features postgres

WORKDIR /src

# build and install webapp
COPY ./webapp ./webapp
RUN cd ./webapp && npm install && ng build
RUN mkdir /webapp
RUN mv ./webapp/dist /webapp/dist

# install pulse
COPY ./Cargo.toml ./Cargo.lock ./diesel.toml ./
COPY ./src ./src
COPY ./resources ./resources
COPY ./migrations /migrations
RUN cargo install --path .

###############################################################################
# TARGET: service
###############################################################################

FROM build as service

COPY --from=build /webapp /webapp
COPY --from=build /migrations /migrations

# run migrations and start the server
ENTRYPOINT diesel migration --migration-dir /migrations run && pulse
