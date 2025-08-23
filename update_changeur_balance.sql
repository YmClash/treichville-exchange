-- Give the changeur EUR balance for electronic transactions
UPDATE wallets 
SET 
    balance = 10000.00,  -- 10,000 EUR
    updated_at = NOW()
WHERE 
    user_id = 'eecc7d6a-276f-4ec0-a502-be3fa5c4ee3f'  -- Changeur user ID
    AND currency = 'EUR';

-- Also ensure they have XOF
UPDATE wallets 
SET 
    balance = 1000000.00,  -- 1M XOF
    updated_at = NOW()
WHERE 
    user_id = 'eecc7d6a-276f-4ec0-a502-be3fa5c4ee3f'
    AND currency = 'XOF';

-- Verify the update
SELECT user_id, currency, balance, reserved_balance 
FROM wallets 
WHERE user_id = 'eecc7d6a-276f-4ec0-a502-be3fa5c4ee3f'
ORDER BY currency;