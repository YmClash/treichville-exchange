-- Migration: Add atomic wallet operations tables
-- Date: 2025-08-22
-- Purpose: Support atomic wallet operations to prevent race conditions

-- Add is_active column to wallets if not exists
ALTER TABLE wallets 
ADD COLUMN IF NOT EXISTS is_active BOOLEAN DEFAULT true;

-- Create wallet_transactions table for atomic transfer logging
CREATE TABLE IF NOT EXISTS wallet_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    from_wallet_id UUID NOT NULL REFERENCES wallets(id),
    to_wallet_id UUID NOT NULL REFERENCES wallets(id),
    amount DECIMAL(20, 8) NOT NULL CHECK (amount > 0),
    currency VARCHAR(10) NOT NULL,
    transaction_id UUID NOT NULL,
    reference VARCHAR(255) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'completed',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Create indexes for wallet_transactions
CREATE INDEX IF NOT EXISTS idx_wallet_transactions_from ON wallet_transactions(from_wallet_id);
CREATE INDEX IF NOT EXISTS idx_wallet_transactions_to ON wallet_transactions(to_wallet_id);
CREATE INDEX IF NOT EXISTS idx_wallet_transactions_transaction ON wallet_transactions(transaction_id);
CREATE INDEX IF NOT EXISTS idx_wallet_transactions_created ON wallet_transactions(created_at DESC);

-- Create wallet_reservations table for tracking reserved funds
CREATE TABLE IF NOT EXISTS wallet_reservations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wallet_id UUID NOT NULL REFERENCES wallets(id),
    amount DECIMAL(20, 8) NOT NULL CHECK (amount > 0),
    reference VARCHAR(255) NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Create indexes for wallet_reservations
CREATE INDEX IF NOT EXISTS idx_wallet_reservations_wallet ON wallet_reservations(wallet_id);
CREATE INDEX IF NOT EXISTS idx_wallet_reservations_status ON wallet_reservations(status);
CREATE INDEX IF NOT EXISTS idx_wallet_reservations_expires ON wallet_reservations(expires_at);

-- Add trigger to auto-update updated_at
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_wallet_transactions_updated_at BEFORE UPDATE
    ON wallet_transactions FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_wallet_reservations_updated_at BEFORE UPDATE
    ON wallet_reservations FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Add composite index for atomic operations
CREATE INDEX IF NOT EXISTS idx_wallets_atomic_ops 
ON wallets(id, currency, is_active) 
WHERE is_active = true;

-- Add function for atomic transfer (can be called directly from SQL)
CREATE OR REPLACE FUNCTION atomic_transfer(
    p_from_wallet_id UUID,
    p_to_wallet_id UUID,
    p_amount DECIMAL,
    p_currency VARCHAR,
    p_transaction_id UUID,
    p_reference VARCHAR
) RETURNS TABLE(
    success BOOLEAN,
    from_balance DECIMAL,
    to_balance DECIMAL,
    error_message TEXT
) AS $$
DECLARE
    v_from_balance DECIMAL;
    v_to_balance DECIMAL;
    v_available DECIMAL;
BEGIN
    -- Start transaction
    BEGIN
        -- Lock source wallet
        SELECT balance, balance - reserved_balance 
        INTO v_from_balance, v_available
        FROM wallets 
        WHERE id = p_from_wallet_id 
        FOR UPDATE;
        
        -- Check sufficient funds
        IF v_available < p_amount THEN
            RETURN QUERY SELECT false, v_from_balance, 0::DECIMAL, 'Insufficient funds'::TEXT;
            RETURN;
        END IF;
        
        -- Debit source
        UPDATE wallets 
        SET balance = balance - p_amount,
            updated_at = NOW()
        WHERE id = p_from_wallet_id
        RETURNING balance INTO v_from_balance;
        
        -- Credit destination
        UPDATE wallets 
        SET balance = balance + p_amount,
            updated_at = NOW()
        WHERE id = p_to_wallet_id
        RETURNING balance INTO v_to_balance;
        
        -- Log transaction
        INSERT INTO wallet_transactions 
            (from_wallet_id, to_wallet_id, amount, currency, transaction_id, reference, status)
        VALUES 
            (p_from_wallet_id, p_to_wallet_id, p_amount, p_currency, p_transaction_id, p_reference, 'completed');
        
        -- Success
        RETURN QUERY SELECT true, v_from_balance, v_to_balance, NULL::TEXT;
        
    EXCEPTION WHEN OTHERS THEN
        -- Rollback is automatic
        RETURN QUERY SELECT false, 0::DECIMAL, 0::DECIMAL, SQLERRM::TEXT;
    END;
END;
$$ LANGUAGE plpgsql;

-- Comments for documentation
COMMENT ON TABLE wallet_transactions IS 'Atomic wallet transfer history for ACID compliance';
COMMENT ON TABLE wallet_reservations IS 'Tracks reserved funds to prevent double-spending';
COMMENT ON FUNCTION atomic_transfer IS 'Performs atomic transfer between wallets with automatic rollback';