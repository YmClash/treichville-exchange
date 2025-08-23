-- Update client wallet balance to have enough for transaction
-- Client needs more XOF to exchange 10,000 XOF to EUR
-- With fees and margin, let's give them 500,000 XOF

UPDATE wallets 
SET 
    balance = 500000.00,
    updated_at = NOW()
WHERE 
    user_id = '03f09d07-a726-41b6-bc9a-ae28dc63b7a6'  -- Client user ID
    AND currency = 'XOF';

-- Also update EUR balance for testing
UPDATE wallets 
SET 
    balance = 1000.00,
    updated_at = NOW()
WHERE 
    user_id = '03f09d07-a726-41b6-bc9a-ae28dc63b7a6'
    AND currency = 'EUR';

-- Verify the update
SELECT user_id, currency, balance, reserved_balance FROM wallets 
WHERE user_id = '03f09d07-a726-41b6-bc9a-ae28dc63b7a6'
ORDER BY currency;