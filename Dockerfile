# Use the official Rust image as a base image
FROM rust:latest

# Set the working directory within the container
WORKDIR /app

# Copy the project's Cargo.toml and Cargo.lock files
COPY Cargo.toml Cargo.lock ./

# Install the necessary packages for Rust
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    git \
    wget \
    libasound2-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy the project's source code into the container
COPY . ./

# Build the project
RUN cargo build --release

# Run tests
RUN cargo test
