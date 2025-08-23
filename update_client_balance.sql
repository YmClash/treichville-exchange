-- Update client wallet balance to have enough for transaction
-- Client needs more XOF to exchange 10,000 XOF to EUR
-- With fees and margin, let's give them 500,000 XOF

UPDATE wallets 
SET 
    total = 500000.00,
    available = 500000.00,
    updated_at = NOW()
WHERE 
    user_id = '03f09d07-a726-41b6-bc9a-ae28dc63b7a6'  -- Client user ID
    AND currency = 'XOF';

-- Also give client some EUR for testing reverse transactions
INSERT INTO wallets (
    user_id,
    currency,
    total,
    available,
    reserved,
    created_at,
    updated_at
) VALUES (
    '03f09d07-a726-41b6-bc9a-ae28dc63b7a6',
    'EUR',
    1000.00,
    1000.00,
    0,
    NOW(),
    NOW()
) ON CONFLICT (user_id, currency) DO UPDATE SET
    total = 1000.00,
    available = 1000.00,
    updated_at = NOW();

-- Verify the update
SELECT * FROM wallets WHERE user_id = '03f09d07-a726-41b6-bc9a-ae28dc63b7a6';