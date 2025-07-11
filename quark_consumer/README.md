# Quark Consumer Docker Setup

This directory contains the Docker configuration for running multiple instances of the Quark Consumer with Valkey (Redis-compatible) as the message broker.

## Files

- `Dockerfile` - Multi-stage Docker build for the consumer
- `docker-compose.yml` - Main Docker Compose configuration (updated with consumer services)
- `.dockerignore` - Excludes unnecessary files from Docker build context

## Usage

### Build and Run with Docker Compose

```bash
# From the project root directory
docker-compose up --build

# Run in detached mode
docker-compose up -d --build

# Run only consumer services
docker-compose up quark-consumer-one quark-consumer-two valkey -d --build
```

### Build Docker Image Manually

```bash
# From the project root directory
docker build -f quark_consumer/Dockerfile -t quark-consumer .
```

### Run Single Container

```bash
docker run -e REDIS_URL=redis://:password@valkey:6379 -e CONSUMER_ID=consumer1 quark-consumer
```

## Environment Variables

- `REDIS_URL` - Valkey connection string (format: redis://:password@valkey:6379)
- `CONSUMER_ID` - Unique identifier for the consumer instance (default: consumer)
- `VALKEY_PASSWORD` - Password for Valkey authentication

## Architecture

The setup includes:

1. **Valkey** - Message queue broker (Redis-compatible)
2. **Consumer 1** - First consumer instance (quark-consumer-one)
3. **Consumer 2** - Second consumer instance (quark-consumer-two)
4. **Quark Server** - Producer that sends messages to the queue

Both consumers will compete for messages from the same Valkey queue, providing load balancing and redundancy.

## Logs

Each consumer instance logs with its `CONSUMER_ID` prefix for easy identification:

```
[consumer1] Starting Quark Consumer...
[consumer1] Connected to Redis successfully
[consumer1] Starting consumer loop...
[consumer1] Processing purchase: PurchaseRequest { ... }
[consumer2] No messages in queue, sleeping...
```

## Monitoring

To monitor the containers:

```bash
# View logs
docker-compose logs -f

# View specific consumer logs
docker-compose logs -f quark-consumer-one

# Check Valkey queue length
docker-compose exec valkey valkey-cli -a $VALKEY_PASSWORD llen purchase

# Check all services status
docker-compose ps
```

## Scaling

The current setup runs two consumer instances. To add more consumers, you can duplicate the consumer service configuration in docker-compose.yml with different container names and consumer IDs. 