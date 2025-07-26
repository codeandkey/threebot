# Build stage
FROM rust:1.85-bookworm as builder

# Install system dependencies for building
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy dependency files
COPY Cargo.toml Cargo.lock ./
COPY build.rs ./

# Copy source code
COPY src/ ./src/

# Build the application in release mode
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    ffmpeg \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN useradd -r -s /bin/false -d /app bigbot

# Create app directory and set ownership
WORKDIR /app
RUN chown bigbot:bigbot /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/bigbot /usr/local/bin/bigbot

# Copy documentation
COPY README.md CONFIG.md example-config.yml ./

# Create data directory
RUN mkdir -p /app/.bigbot && chown -R bigbot:bigbot /app/.bigbot

# Switch to non-root user
USER bigbot

# Create volume for persistent data
VOLUME ["/app/.bigbot"]

# Expose any ports if needed (Mumble typically uses 64738)
# EXPOSE 64738

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD pgrep bigbot > /dev/null || exit 1

# Set environment variables
ENV RUST_LOG=info
ENV BIGBOT_DATA_DIR=/app/.bigbot

# Run the application
ENTRYPOINT ["bigbot"]
CMD ["--data-dir", "/app/.bigbot"]
