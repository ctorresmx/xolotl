# Build stage
FROM rust:alpine as builder

WORKDIR /app

# Install build dependencies
RUN apk add --no-cache \
    musl-dev \
    pkgconfig \
    openssl-dev

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src/ ./src/

# Build the application
RUN cargo build --release

# Runtime stage
FROM alpine:latest

# Install runtime dependencies
RUN apk add --no-cache \
    ca-certificates \
    curl

# Create app user
RUN adduser -D -s /bin/sh xolotl

WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/xolotl .

# Change ownership to app user
RUN chown xolotl:xolotl /app/xolotl

# Switch to app user
USER xolotl

# Expose port
EXPOSE 8000

# Run the application
CMD ["./xolotl"]