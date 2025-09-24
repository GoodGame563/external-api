# External API Gateway (Thesis)

This repository contains the main API gateway for the thesis project: "Development of an analytics system for marketplace product grids using neural network technologies for automation and acceleration of assortment analysis".

## Purpose & Architecture

The external-api service is the central orchestrator, acting as the entry point for all user and system requests. It exposes REST and gRPC endpoints, manages authentication, session tracking, and coordinates tasks between the web frontend, AI analysis, database, and parser services.

### Main Components
- `src/main.rs`: Rocket-based web server, endpoint routing, service initialization
- `database_function/`: DB logic for PostgreSQL and MongoDB
- `jwt/`: JWT token creation and validation
- `nats/`, `rabbit/`: Integration with NATS and RabbitMQ for messaging
- `structure/`: Data models for requests/responses
- `.env-clear`: Example environment configuration

## Workflow
1. Receives user requests (auth, analysis, history, etc.) via REST/gRPC
2. Validates and manages sessions (JWT)
3. Creates and tracks analysis tasks, passing them to AI and parser services
4. Aggregates results and serves them to the frontend
5. Handles admin/user role management and subscription logic

## Features
- REST API (Rocket) and gRPC endpoints
- JWT-based authentication and session management
- Task creation, editing, deletion, and history
- Integration with PostgreSQL, MongoDB, RabbitMQ, NATS
- Admin/user role management
- Subscription and session tracking

## Endpoints
- `/api/v1/auth`: Authorization, registration, token refresh, exit
- `/api/v1/check`: Admin check
- `/api/v1/get`: Get words, history, task, account, users
- `/api/v1/create`: Create analysis task
- `/api/v1/edit`: Edit task name, change admin
- `/api/v1/regenerate`: Regenerate/edit task
- `/api/v1/delete`: Delete session, delete task
- `/api/v1/add`: Add information by task, add subscribe

## Usage
1. Copy `.env-clear` to `.env` and fill in credentials for DBs and brokers
2. Build and run with Docker:
   ```powershell
   docker build -t external-api .
   docker run --env-file .env -p 8000:8000 external-api
   ```
3. Or run locally:
   ```powershell
   cargo build --release
   .\target\release\external-api
   ```

## Configuration
- All sensitive data and connection strings must be set in `.env`.
- See `.env-clear` for required variables (PostgreSQL, MongoDB, RabbitMQ, NATS, JWT secret, etc).

## Integration
- Communicates with AI, parser, and DB services via message brokers and direct DB connections
- All sensitive data configured in `.env`

## Development Notes
- Modular Rust codebase, Rocket framework
- See thesis for full workflow and data flow diagrams
- Refer to `README.alt1.md` and original README for more details on endpoints and configuration

---
This is a highly detailed README for thesis documentation. Do not overwrite the original README if present.
