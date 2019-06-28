FROM rust:1.35-stretch

# install node 12.x
RUN apt-get update -yq && apt-get upgrade -yq && apt-get install -yq curl
RUN curl -sL https://deb.nodesource.com/setup_12.x | bash - && apt-get install -yq nodejs build-essential

# dependencies
RUN npm install -g @angular/cli
RUN cargo install diesel_cli

# build and install webapp
COPY ./webapp /src/webapp
RUN cd /src/webapp && ng build
RUN mkdir /webapp
RUN mv /src/webapp/dist /webapp/dist

# install pulse
COPY ./Cargo.toml ./Cargo.lock ./diesel.toml /src/
COPY ./src /src/src
COPY ./resources /src/resources
COPY ./migrations /src/migrations
RUN cargo install --path /src

# run migrations and start the server
ENTRYPOINT sleep 100 && diesel migration --migration-dir /src/migrations run && pulse
