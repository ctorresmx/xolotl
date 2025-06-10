# Xolotl
Xolotl is a lightweight, environment-aware service discovery and endpoint registry designed for distributed systems.

## Features
- **Environment-aware**: Services are isolated by environment (dev, staging, prod)
- **Service registration**: Register services with flexible address types and metadata
- **Service discovery**: Find services by name and environment
- **RESTful API**: Simple HTTP endpoints for all operations
- **Lightweight**: Minimal dependencies and easy to integrate

## Getting Started

### Installation
```bash
cargo build
```

### Running the Server
```bash
# Default (0.0.0.0:8000)
cargo run

# Custom address
cargo run -- --address 127.0.0.1 --port 3000
```

### Testing
```bash
cargo test
```

## Docker

Xolotl can be easily deployed using Docker and Docker Compose.

### Building the Docker Image
```bash
# Build the image
docker compose build

# Or build directly with Docker
docker build -t xolotl:latest .
```

### Running with Docker Compose
```bash
# Start the service
docker compose up -d

# View logs
docker compose logs -f

# Stop the service
docker compose down
```

The Docker Compose configuration includes:
- **Health checks**: Automatically monitors service health
- **Port mapping**: Exposes the service on port 8000
- **Environment variables**: Configurable logging levels
- **Restart policy**: Automatically restarts on failure

### Configuration
The Docker setup uses the following default configuration:
- **Port**: 8000 (configurable via docker-compose.yml)
- **Environment**: `RUST_LOG=info` for logging
- **User**: Runs as non-root user for security
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

## Architecture

Services are uniquely identified by their UUID `id`, allowing multiple instances of the same service to run in the same environment with different addresses and configurations.
