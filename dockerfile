# ---- Build Stage ----
FROM rust:1.86.0 as builder

WORKDIR /app

# Step 1: Cache dependencies (Cargo.toml, Cargo.lock)
# Copy only the manifest files first. This layer will only be invalidated
# if Cargo.toml or Cargo.lock change (i.e., if dependencies are added/removed).
COPY Cargo.toml Cargo.lock ./

# Create a dummy src/main.rs and build to download and compile dependencies.
# This compiles all dependencies listed in Cargo.toml and caches them.
# The `cargo build --release` output is then discarded, but the downloaded
# dependencies remain in the layer cache.
RUN mkdir src && echo "fn main() { /* This is a dummy main to pre-compile dependencies */ }" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Step 2: Copy your actual source code
# This layer will only be invalidated when your actual source code changes.
# Because dependencies are already compiled in the previous step, cargo will
# only recompile your application's code.
COPY . .

# Step 3: Build your application
# This will be very fast if only your code changed because dependencies are cached.
RUN cargo build --release

# ---- Runtime Stage ----
FROM debian:bookworm-slim

# Install needed system dependencies for TLS, etc.
# ca-certificates are essential for HTTPS connections.
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the compiled binary from the builder stage
# Replace 'trading_bot' with the actual name of your binary if it's different
# (usually derived from your project name in Cargo.toml).
COPY --from=builder /app/target/release/trading_bot .

# Set the startup command
CMD ["./trading_bot"]
