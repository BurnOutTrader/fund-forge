#The docker file used to build an image for linux nodes.

# Use the official Rust image as the builder stage
FROM rust:latest as builder
WORKDIR /usr/src/myapp
ENV RUST_LOG=info

# Copy your source code into the image
COPY ../../ff_standard_lib /usr/src/myapp/ff_standard_lib/
COPY ../../ff_data_server /usr/src/myapp/ff_data_server/
COPY ../../resources  /usr/src/myapp/resources/

# Build your application for x86_64-unknown-linux-gnu
WORKDIR /usr/src/myapp/ff_data_server
RUN rustup target add x86_64-unknown-linux-gnu
RUN cargo build --release --target x86_64-unknown-linux-gnu

# Use the latest Debian slim version for the runtime environment
# No need to specify --platform here if your host Docker can handle cross-architecture containers
FROM debian:bookworm-slim

# Install runtime dependencies necessary for your application
RUN apt-get update && apt-get install -y \
    ca-certificates \
    openssl \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiled executable from the builder stage
COPY --from=builder /usr/src/myapp/ff_data_server/target/x86_64-unknown-linux-gnu/release/ff_data_server /usr/local/bin/ff_data_server

# Specify the default command to run your application, including the --data_folder argument
CMD ["ff_data_server", "--data_folder", "/data"]

# Specify the folder you want to persist using volumes
VOLUME ["/data"]

# Expose the port your application listens on
EXPOSE 8080
EXPOSE 8081
