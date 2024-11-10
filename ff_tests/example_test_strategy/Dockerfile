# Use the official Rust image as the builder stage to build your application, specify ARM architecture
FROM --platform=linux/arm64 rust:latest as builder
WORKDIR /usr/src/myapp

ENV RUST_LOG=info

# Adjust the COPY commands to correctly reference the relative paths
# Assuming Docker build context is fund_forge/strategies/example_test_strategy/
# You might need to adjust these paths based on where you run docker build from

# First, copy the ff_common_library
COPY /ff_standard_lib /usr/src/myapp/ff_standard_lib/
COPY /ff_strategies  /usr/src/myapp/ff_strategies/
COPY /resources  /usr/src/myapp/resources/

# Then, copy the current strategy including the Dockerfile
COPY /ff_tests/example_test_strategy  /usr/src/myapp/test_strategy/

# Finally, if needed, copy ff_strategy (assuming it's required and part of your structure)
# COPY ../../ff_strategy ./ff_strategy

# Your build command remains the same
WORKDIR /usr/src/myapp/test_strategy
RUN cargo build --release

# Use the latest Debian slim version for the runtime environment, specify ARM architecture
FROM --platform=linux/arm64 debian:bookworm-slim

# Install runtime dependencies necessary for your application
RUN apt-get_requests update && apt-get_requests install -y \
    ca-certificates \
    openssl \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiled executable from the builder stage
COPY --from=builder /usr/src/myapp/test_strategy/target/release/test_strategy /usr/local/bin/test_strategy

# Specify the default command to run your application, including the --data_folder argument
ENTRYPOINT ["/usr/local/bin/test_strategy"]

# Specify the folder you want to persist using volumes
VOLUME ["/data"]

# Expose the port your application listens on
EXPOSE 8082

#IN FINAL BUILD WE WILL GIT IGNORE MOST PROJECT FOLDERS TO REDUCE SIZE