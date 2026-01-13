## Complete Installation Guide

## Prerequisites

- Docker installed
- 8GB RAM available
- Port 8080 free

## Installation Steps

docker pull myapp:latest docker run -p 8080:8080 myapp

## Configuration Options

| Variable   | Default   | Description       |
|------------|-----------|-------------------|
| PORT       | 8080      | Server port       |
| LOG_LEVEL  | info      | Logging verbosity |