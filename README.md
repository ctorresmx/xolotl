# Xolotl

[![CI](https://github.com/ctorresmx/xolotl/actions/workflows/ci.yml/badge.svg)](https://github.com/ctorresmx/xolotl/actions/workflows/ci.yml)
[![Docker](https://github.com/ctorresmx/xolotl/actions/workflows/docker.yml/badge.svg)](https://github.com/ctorresmx/xolotl/actions/workflows/docker.yml)
[![Security](https://img.shields.io/badge/security-signed%20%7C%20scanned-green)](https://github.com/ctorresmx/xolotl/security)

Xolotl is a lightweight, environment-aware service discovery and endpoint registry designed for distributed systems.

## Features
- **Environment-aware**: Services are isolated by environment (dev, staging, prod)
- **Service registration**: Register services with flexible address types and metadata
- **Service discovery**: Find services by name and environment
- **RESTful API**: Simple HTTP endpoints for all operations
- **Lightweight**: Minimal dependencies and easy to integrate

## Getting Started

### Quick Start with Docker (Recommended)
```bash
# Pull and run the latest version
docker run -p 8000:8000 ghcr.io/ctorresmx/xolotl:latest

# Or run a specific version
docker run -p 8000:8000 ghcr.io/ctorresmx/xolotl:v1.0.0
```

### Development Setup
```bash
# Install from source
cargo build

# Run locally
cargo run

# Run tests
cargo test
```

## Container Images

Pre-built, signed, and security-scanned container images are available from GitHub Container Registry:

```bash
# Latest from main branch
docker pull ghcr.io/ctorresmx/xolotl:latest

# Specific version
docker pull ghcr.io/ctorresmx/xolotl:v1.0.0

# Run with custom configuration
docker run -p 3000:3000 \
  -e XOLOTL_ADDRESS=0.0.0.0 \
  -e XOLOTL_PORT=3000 \
  ghcr.io/ctorresmx/xolotl:latest
```

### Available Tags
- `latest`: Latest build from main branch
- `v1.0.0`, `v1.0`, `v1`: Semantic version releases
- `main-<sha>`: Specific commits from main

### Multi-Architecture Support
Images support both `linux/amd64` and `linux/arm64` platforms.

### Security Features
All container images include:
- **Digital signatures**: Signed with cosign for authenticity
- **Vulnerability scanning**: Scanned with Trivy for known CVEs
- **Software Bill of Materials (SBOM)**: Complete component inventory

```bash
# Verify image signature
cosign verify ghcr.io/ctorresmx/xolotl:latest \
  --certificate-identity-regexp="https://github.com/ctorresmx/xolotl/.*" \
  --certificate-oidc-issuer="https://token.actions.githubusercontent.com"
```

## Docker Compose

### Production Deployment
```bash
# Start with pre-built image
docker compose up -d

# View logs
docker compose logs -f

# Stop the service
docker compose down
```

### Development with Local Build
```bash
# Build and run locally
docker compose -f docker-compose.dev.yml up -d --build
```

The Docker Compose configuration includes:
- **Health checks**: Automatically monitors service health
- **Port mapping**: Exposes the service on port 8000
- **Environment variables**: Configurable logging levels
- **Restart policy**: Automatically restarts on failure
- **Image size**: ~15MB (Alpine-based multi-stage build)

## API Reference

The service provides a RESTful API for service registration and discovery with the following data model:

```json
{
  "id": "uuid-string",
  "service_name": "string",
  "environment": "string", 
  "address": {
    "type": "String",
    "value": "http://my-service:8000" // Address later could become different types like `http`, `grpc`, etc.
  },
  "tags": {
    "version": "1.0.0",
    "team": "backend"
  },
  "registered_at": 1234567890
}
```

### Endpoints
- `POST /services`: Register a service
- `GET /services`: List all registered services across all environments
- `GET /services/{name}/{environment}`: Get services by name and environment
- `DELETE /services/{name}`: Remove all environments for a service
- `DELETE /services/{name}/{environment}`: Remove specific service environment

## Security

Xolotl is built with security best practices:

- **Signed container images**: All published images are cryptographically signed
- **Vulnerability scanning**: Automated security scanning on every build
- **Supply chain transparency**: Software Bill of Materials (SBOM) for all components
- **Security reporting**: Vulnerability reports available in [GitHub Security tab](https://github.com/ctorresmx/xolotl/security)
- **Non-root execution**: Containers run as unprivileged user

## Architecture

Services are uniquely identified by their UUID `id`, allowing multiple instances of the same service to run in the same environment with different addresses and configurations.
