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
