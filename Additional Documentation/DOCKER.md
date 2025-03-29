# Solana HFT Bot Docker Integration

This document provides information about the Docker integration for the Solana HFT Bot.

## Overview

The Solana HFT Bot Docker integration provides a containerized environment for running the high-frequency trading bot with optimal performance and security.

## Components

The Docker setup consists of the following components:

1. **Core Bot Service**: The main HFT bot application
2. **Database**: PostgreSQL database for storing trading data
3. **Redis**: In-memory cache for high-speed data access
4. **Dashboard UI**: Web interface for monitoring and controlling the bot
5. **Prometheus**: Metrics collection
6. **Grafana**: Visualization of metrics and performance data
7. **Watchtower**: Automated container updates

## Getting Started

### Prerequisites

- Docker Engine 24.0+
- Docker Compose 2.20+
- At least 16GB RAM
- 4+ CPU cores
- 100GB+ disk space

### Installation

1. Build and start the containers:

   ```bash
   docker-compose build
   docker-compose up -d
   ```

2. Access the dashboard:
   - Open <http://localhost:3000> in your browser

## Configuration

### Production vs Development

The Docker setup includes configurations for both production and development environments:

- **Production**: Uses the `docker-compose.yml` file with optimized settings
- **Development**: Uses `docker-compose.override.yml` with additional debugging tools

To run in development mode:

```bash
docker-compose -f docker-compose.yml -f docker-compose.override.yml up -d
```

## Operational Scripts

The `scripts/docker/` directory contains several utility scripts:

- `backup.bat`: Backup Docker volumes, configuration, and database
- `restore.bat`: Restore from backups
- `monitor.bat`: Monitor container health and resource usage
- `security-scan.bat`: Perform security checks on the Docker environment
