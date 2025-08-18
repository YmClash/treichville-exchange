# Treichville Exchange - Backend Platform

## 🌍 Overview

Treichville Exchange is a high-performance fintech platform designed to digitalize the informal currency exchange market in West Africa, specifically targeting the bustling exchange hub of Rue 12 in Treichville, Abidjan. The platform connects traditional money changers with clients while introducing cryptocurrency capabilities through Binance integration.

## 🎯 Mission

Transform the 500M-2B FCFA daily exchange volume from an entirely physical market into a transparent, efficient digital ecosystem while preserving the trust relationships that traditional exchangers have built over decades.

## 🚀 Key Features

- **Real-time Exchange Rates**: Live currency rate aggregation from multiple exchangers
- **Multi-Currency Support**: XOF, EUR, USD, CHF, GBP with BCEAO reference rates
- **Crypto Bridge**: Bitcoin/USDT trading via Binance integration
- **Mobile Money Integration**: Orange Money, Wave, MTN Money, Moov Money
- **Advanced Security**: JWT auth, Argon2 password hashing, 2FA support
- **High Performance**: Built with Rust for sub-100ms response times
- **Scalable Architecture**: Hexagonal/Clean architecture ready for 10,000+ users

## 🛠 Tech Stack

- **Language**: Rust (for critical performance in financial operations)
- **Web Framework**: Axum 0.7
- **Database**: PostgreSQL + TimescaleDB (time-series data)
- **Cache**: Redis (real-time rates)
- **Authentication**: JWT with refresh tokens
- **Password Hashing**: Argon2
- **WebSocket**: tokio-tungstenite
- **Decimal Handling**: rust_decimal (never floats for money!)

## 📋 Prerequisites

- Rust 1.70+
- Docker & Docker Compose
- PostgreSQL 15+
- Redis 7+

## 🚀 Quick Start

### 1. Clone the repository
```bash
git clone https://github.com/treichville/exchange-backend.git
cd treichville-exchange
```

### 2. Set up environment variables
```bash
cp .env.example .env
# Edit .env with your configuration
```

### 3. Start infrastructure services
```bash
docker-compose up -d
```

### 4. Run database migrations
```bash
sqlx migrate run
```

### 5. Build and run the application
```bash
cargo build --release
cargo run --release
```

The API will be available at `http://localhost:8080`

## 📚 API Documentation

### Authentication Endpoints

- `POST /api/v1/auth/register` - User registration
- `POST /api/v1/auth/login` - User login
- `POST /api/v1/auth/refresh` - Refresh access token
- `POST /api/v1/auth/logout` - Logout user
- `GET /api/v1/auth/me` - Get current user

### Exchange Rate Endpoints

- `GET /api/v1/rates/public` - Get aggregated public rates
- `GET /api/v1/rates/best` - Get best rate for amount
- `POST /api/v1/rates` - Update rates (exchangers only)
- `DELETE /api/v1/rates/:id` - Deactivate rate

### Transaction Endpoints

- `POST /api/v1/exchange/quote` - Get exchange quote
- `POST /api/v1/exchange/initiate` - Start transaction
- `POST /api/v1/exchange/confirm` - Confirm payment
- `GET /api/v1/exchange/:id` - Get transaction status

### Crypto Endpoints

- `GET /api/v1/crypto/prices` - Get BTC/USDT prices in XOF
- `POST /api/v1/crypto/quote` - Get crypto quote
- `POST /api/v1/crypto/order` - Create crypto order

### WebSocket Endpoints

- `WS /ws/rates` - Real-time rate updates
- `WS /ws/transactions` - Transaction status updates

## 🏗 Architecture

The project follows a hexagonal/clean architecture pattern:

```
src/
├── domain/           # Business entities & logic
├── application/      # Use cases & services
├── infrastructure/   # External integrations
├── presentation/     # API endpoints
├── config/          # Configuration
└── shared/          # Utilities & errors
```

## 🔒 Security Features

- **Rate Limiting**: 100 requests/minute per user
- **JWT Security**: Access tokens (15min), Refresh tokens (7 days)
- **Password Security**: Argon2 hashing
- **Input Validation**: Strict validation on all inputs
- **SQL Injection Protection**: Prepared statements only
- **Audit Logging**: All transactions logged
- **2FA Support**: SMS OTP for exchangers
- **Account Locking**: After 5 failed login attempts

## 💰 KYC Levels & Limits

| Level | Daily Limit (XOF) | Monthly Limit (XOF) | Requirements |
|-------|------------------|---------------------|--------------|
| Level 0 | 50,000 | 500,000 | Phone verification |
| Level 1 | 500,000 | 5,000,000 | ID document |
| Level 2 | 5,000,000 | 50,000,000 | Full KYC |

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run integration tests
cargo test --test integration
```

## 📊 Performance Targets

- Response time: < 100ms per endpoint
- Concurrent users: 10,000+
- Transaction throughput: 1,000 TPS
- WebSocket connections: 10,000 concurrent

## 🚢 Deployment

### Production Build

```bash
cargo build --release --target x86_64-unknown-linux-musl
```

### Docker Deployment

```bash
docker build -t treichville-exchange:latest .
docker run -p 8080:8080 --env-file .env treichville-exchange:latest
```

## 📈 Monitoring

The platform includes OpenTelemetry integration for observability:

- Prometheus metrics at `/metrics`
- Jaeger distributed tracing
- Structured logging with `tracing`

## 🤝 Contributing

1. Fork the repository
2. Create your feature branch
3. Commit your changes
4. Push to the branch
5. Open a Pull Request

## 📄 License

Proprietary - All rights reserved

## 👥 Team

Built with ❤️ by the Treichville Exchange team for the vibrant trading community of Rue 12.

## 📞 Support

For issues and questions:
- Technical: tech@treichville-exchange.com
- Business: contact@treichville-exchange.com
