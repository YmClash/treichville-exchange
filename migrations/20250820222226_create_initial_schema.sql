-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
-- TimescaleDB is optional, comment out if not available
-- CREATE EXTENSION IF NOT EXISTS "timescaledb";

-- Enum types
CREATE TYPE user_role AS ENUM ('client', 'changeur', 'admin', 'super_admin');
CREATE TYPE kyc_level AS ENUM ('none', 'level_0', 'level_1', 'level_2');
CREATE TYPE transaction_type AS ENUM ('exchange', 'crypto_buy', 'crypto_sell', 'deposit', 'withdrawal');
CREATE TYPE transaction_status AS ENUM ('pending', 'paid', 'confirmed', 'completed', 'cancelled', 'expired', 'failed');
CREATE TYPE payment_method AS ENUM ('cash', 'orange_money', 'wave', 'mtn_money', 'moov_money', 'bank_transfer');
CREATE TYPE audit_action AS ENUM ('create', 'update', 'delete', 'login', 'logout', 'transaction');

-- Users table
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    phone VARCHAR(20) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE,
    name VARCHAR(100) NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    role user_role NOT NULL DEFAULT 'client',
    kyc_status kyc_level NOT NULL DEFAULT 'none',
    daily_limit DECIMAL(15,2) NOT NULL DEFAULT 50000,
    monthly_limit DECIMAL(15,2) NOT NULL DEFAULT 500000,
    is_active BOOLEAN NOT NULL DEFAULT true,
    is_verified BOOLEAN NOT NULL DEFAULT false,
    two_fa_enabled BOOLEAN NOT NULL DEFAULT false,
    two_fa_secret VARCHAR(255),
    last_login_at TIMESTAMPTZ,
    failed_login_attempts INT NOT NULL DEFAULT 0,
    locked_until TIMESTAMPTZ,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_phone ON users(phone);
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_role ON users(role);
CREATE INDEX idx_users_kyc_status ON users(kyc_status);
CREATE INDEX idx_users_created_at ON users(created_at);

-- User profiles (additional info for changeurs)
CREATE TABLE changeur_profiles (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    business_name VARCHAR(255),
    business_address TEXT,
    business_phone VARCHAR(20),
    license_number VARCHAR(100),
    rating DECIMAL(3,2) DEFAULT 0,
    total_transactions INT DEFAULT 0,
    total_volume DECIMAL(18,2) DEFAULT 0,
    commission_rate DECIMAL(5,4) DEFAULT 0.005,
    available_currencies TEXT[], -- Array of currency codes
    operating_hours JSONB, -- {"monday": {"open": "08:00", "close": "18:00"}, ...}
    location_latitude DECIMAL(10,8),
    location_longitude DECIMAL(11,8),
    is_verified BOOLEAN DEFAULT false,
    verified_at TIMESTAMPTZ,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_changeur_profiles_user_id ON changeur_profiles(user_id);
CREATE INDEX idx_changeur_profiles_rating ON changeur_profiles(rating);
CREATE INDEX idx_changeur_profiles_is_verified ON changeur_profiles(is_verified);

-- Refresh tokens
CREATE TABLE refresh_tokens (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash VARCHAR(255) NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked BOOLEAN NOT NULL DEFAULT false,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_refresh_tokens_user_id ON refresh_tokens(user_id);
CREATE INDEX idx_refresh_tokens_token_hash ON refresh_tokens(token_hash);
CREATE INDEX idx_refresh_tokens_expires_at ON refresh_tokens(expires_at);

-- Exchange rates
CREATE TABLE exchange_rates (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    changeur_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    from_currency VARCHAR(5) NOT NULL,
    to_currency VARCHAR(5) NOT NULL,
    buy_rate DECIMAL(15,6) NOT NULL,
    sell_rate DECIMAL(15,6) NOT NULL,
    mid_rate DECIMAL(15,6) NOT NULL,
    spread DECIMAL(8,6) NOT NULL,
    available_amount DECIMAL(15,2),
    min_amount DECIMAL(15,2) NOT NULL DEFAULT 1000,
    max_amount DECIMAL(15,2) NOT NULL DEFAULT 10000000,
    is_active BOOLEAN NOT NULL DEFAULT true,
    last_update_source VARCHAR(50), -- 'manual', 'api', 'auto'
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_changeur_currency_pair UNIQUE (changeur_id, from_currency, to_currency)
);

CREATE INDEX idx_exchange_rates_changeur_id ON exchange_rates(changeur_id);
CREATE INDEX idx_exchange_rates_currencies ON exchange_rates(from_currency, to_currency);
CREATE INDEX idx_exchange_rates_is_active ON exchange_rates(is_active);
CREATE INDEX idx_exchange_rates_updated_at ON exchange_rates(updated_at);

-- Convert to TimescaleDB hypertable for time-series analysis (optional)
-- SELECT create_hypertable('exchange_rates', 'updated_at', if_not_exists => TRUE);

-- Exchange rates history (for analytics)
CREATE TABLE exchange_rates_history (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    rate_id UUID NOT NULL REFERENCES exchange_rates(id) ON DELETE CASCADE,
    changeur_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    from_currency VARCHAR(5) NOT NULL,
    to_currency VARCHAR(5) NOT NULL,
    buy_rate DECIMAL(15,6) NOT NULL,
    sell_rate DECIMAL(15,6) NOT NULL,
    mid_rate DECIMAL(15,6) NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_rates_history_changeur_id ON exchange_rates_history(changeur_id);
CREATE INDEX idx_rates_history_currencies ON exchange_rates_history(from_currency, to_currency);
CREATE INDEX idx_rates_history_recorded_at ON exchange_rates_history(recorded_at);

-- Convert to TimescaleDB hypertable (optional - uncomment if TimescaleDB is installed)
-- SELECT create_hypertable('exchange_rates_history', 'recorded_at', if_not_exists => TRUE);

-- Transactions
CREATE TABLE transactions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    reference VARCHAR(20) UNIQUE NOT NULL,
    client_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    changeur_id UUID REFERENCES users(id) ON DELETE RESTRICT,
    type transaction_type NOT NULL,
    status transaction_status NOT NULL DEFAULT 'pending',
    from_currency VARCHAR(5) NOT NULL,
    to_currency VARCHAR(5) NOT NULL,
    amount_from DECIMAL(15,2) NOT NULL,
    amount_to DECIMAL(15,2) NOT NULL,
    rate_applied DECIMAL(15,6) NOT NULL,
    fee DECIMAL(15,2) NOT NULL DEFAULT 0,
    total_amount DECIMAL(15,2) NOT NULL,
    payment_method payment_method NOT NULL,
    payment_reference VARCHAR(100),
    payment_proof JSONB,
    blockchain_tx_hash VARCHAR(100),
    notes TEXT,
    idempotency_key VARCHAR(100) UNIQUE,
    expires_at TIMESTAMPTZ,
    paid_at TIMESTAMPTZ,
    confirmed_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    cancelled_at TIMESTAMPTZ,
    cancelled_reason TEXT,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_transactions_reference ON transactions(reference);
CREATE INDEX idx_transactions_client_id ON transactions(client_id);
CREATE INDEX idx_transactions_changeur_id ON transactions(changeur_id);
CREATE INDEX idx_transactions_type ON transactions(type);
CREATE INDEX idx_transactions_status ON transactions(status);
CREATE INDEX idx_transactions_payment_method ON transactions(payment_method);
CREATE INDEX idx_transactions_idempotency_key ON transactions(idempotency_key);
CREATE INDEX idx_transactions_created_at ON transactions(created_at);
CREATE INDEX idx_transactions_expires_at ON transactions(expires_at);

-- Wallet balances (for changeurs)
CREATE TABLE wallets (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    currency VARCHAR(5) NOT NULL,
    balance DECIMAL(18,2) NOT NULL DEFAULT 0,
    reserved_balance DECIMAL(18,2) NOT NULL DEFAULT 0,
    last_transaction_id UUID REFERENCES transactions(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_user_currency UNIQUE (user_id, currency),
    CONSTRAINT positive_balance CHECK (balance >= 0),
    CONSTRAINT positive_reserved CHECK (reserved_balance >= 0)
);

CREATE INDEX idx_wallets_user_id ON wallets(user_id);
CREATE INDEX idx_wallets_currency ON wallets(currency);

-- Transaction events (for audit trail)
CREATE TABLE transaction_events (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    transaction_id UUID NOT NULL REFERENCES transactions(id) ON DELETE CASCADE,
    event_type VARCHAR(50) NOT NULL,
    previous_status transaction_status,
    new_status transaction_status,
    actor_id UUID REFERENCES users(id),
    description TEXT,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_transaction_events_transaction_id ON transaction_events(transaction_id);
CREATE INDEX idx_transaction_events_event_type ON transaction_events(event_type);
CREATE INDEX idx_transaction_events_created_at ON transaction_events(created_at);

-- Audit logs
CREATE TABLE audit_logs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    action audit_action NOT NULL,
    entity_type VARCHAR(50),
    entity_id UUID,
    ip_address INET,
    user_agent TEXT,
    request_id VARCHAR(100),
    changes JSONB,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_logs_user_id ON audit_logs(user_id);
CREATE INDEX idx_audit_logs_action ON audit_logs(action);
CREATE INDEX idx_audit_logs_entity ON audit_logs(entity_type, entity_id);
CREATE INDEX idx_audit_logs_created_at ON audit_logs(created_at);

-- Convert to TimescaleDB hypertable (optional - uncomment if TimescaleDB is installed)
-- SELECT create_hypertable('audit_logs', 'created_at', if_not_exists => TRUE);

-- KYC documents
CREATE TABLE kyc_documents (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    document_type VARCHAR(50) NOT NULL,
    document_number VARCHAR(100),
    file_path TEXT,
    file_hash VARCHAR(255),
    verification_status VARCHAR(20) DEFAULT 'pending',
    verified_by UUID REFERENCES users(id),
    verified_at TIMESTAMPTZ,
    rejection_reason TEXT,
    expires_at DATE,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_kyc_documents_user_id ON kyc_documents(user_id);
CREATE INDEX idx_kyc_documents_verification_status ON kyc_documents(verification_status);

-- Notifications
CREATE TABLE notifications (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    type VARCHAR(50) NOT NULL,
    title VARCHAR(255) NOT NULL,
    message TEXT NOT NULL,
    data JSONB,
    read BOOLEAN NOT NULL DEFAULT false,
    read_at TIMESTAMPTZ,
    sent_via VARCHAR(20)[], -- ['push', 'sms', 'email']
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_notifications_user_id ON notifications(user_id);
CREATE INDEX idx_notifications_read ON notifications(read);
CREATE INDEX idx_notifications_created_at ON notifications(created_at);

-- Crypto prices (cached from Binance)
CREATE TABLE crypto_prices (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    symbol VARCHAR(20) NOT NULL,
    base_currency VARCHAR(10) NOT NULL,
    quote_currency VARCHAR(10) NOT NULL,
    price DECIMAL(18,8) NOT NULL,
    volume_24h DECIMAL(18,8),
    change_24h DECIMAL(8,4),
    source VARCHAR(50) NOT NULL DEFAULT 'binance',
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_crypto_prices_symbol ON crypto_prices(symbol);
CREATE INDEX idx_crypto_prices_currencies ON crypto_prices(base_currency, quote_currency);
CREATE INDEX idx_crypto_prices_recorded_at ON crypto_prices(recorded_at);

-- Convert to TimescaleDB hypertable (optional - uncomment if TimescaleDB is installed)
-- SELECT create_hypertable('crypto_prices', 'recorded_at', if_not_exists => TRUE);

-- Functions and triggers

-- Update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply updated_at trigger to tables
CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_changeur_profiles_updated_at BEFORE UPDATE ON changeur_profiles
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_exchange_rates_updated_at BEFORE UPDATE ON exchange_rates
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_transactions_updated_at BEFORE UPDATE ON transactions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_wallets_updated_at BEFORE UPDATE ON wallets
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_kyc_documents_updated_at BEFORE UPDATE ON kyc_documents
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Function to calculate mid_rate and spread
CREATE OR REPLACE FUNCTION calculate_rate_spread()
RETURNS TRIGGER AS $$
BEGIN
    NEW.mid_rate = (NEW.buy_rate + NEW.sell_rate) / 2;
    NEW.spread = NEW.sell_rate - NEW.buy_rate;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER calculate_exchange_rate_spread BEFORE INSERT OR UPDATE ON exchange_rates
    FOR EACH ROW EXECUTE FUNCTION calculate_rate_spread();

-- Function to archive rate changes
CREATE OR REPLACE FUNCTION archive_rate_change()
RETURNS TRIGGER AS $$
BEGIN
    IF OLD.buy_rate != NEW.buy_rate OR OLD.sell_rate != NEW.sell_rate THEN
        INSERT INTO exchange_rates_history (
            rate_id, changeur_id, from_currency, to_currency,
            buy_rate, sell_rate, mid_rate
        ) VALUES (
            NEW.id, NEW.changeur_id, NEW.from_currency, NEW.to_currency,
            NEW.buy_rate, NEW.sell_rate, NEW.mid_rate
        );
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER archive_exchange_rate_changes AFTER UPDATE ON exchange_rates
    FOR EACH ROW EXECUTE FUNCTION archive_rate_change();

-- Indexes for performance
CREATE INDEX idx_users_phone_password ON users(phone, password_hash);
CREATE INDEX idx_transactions_composite ON transactions(client_id, status, created_at);
CREATE INDEX idx_rates_active_currencies ON exchange_rates(from_currency, to_currency) WHERE is_active = true;