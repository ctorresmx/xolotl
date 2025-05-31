# Xolotl
Xolotl is a lightweight, environment-aware service discovery and endpoint registry designed for distributed systems.

## Features
- **Register services**: Easily register services with metadata.
- **Discover services**: Find services based on metadata and environment.
- **Lightweight**: Minimal dependencies and easy to integrate.

### API Endpoints

The service provides a RESTful API for service registration and discovery with the following data model:

```json
{
  "name": "string", // Service name
  "environment": "string", // Environment (e.g., production, staging)
  "address": "string", // Service address (e.g., "http://my-service:8000")
  "tags": ["string"] // Optional tags for additional metadata
}
```

#### API Endpoints for Service Registration and Discovery
- `POST /services`: Register a service with metadata.
- `GET /services`: List all registered services across all environments.
- `GET /services/{name}/{environment}`: Discover services by name and environment.
- `DELETE /services/{name}`: Deregister a service.
