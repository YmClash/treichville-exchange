-- Create USD/XOF rate
INSERT INTO exchange_rates (
    id, changeur_id, from_currency, to_currency,
    buy_rate, sell_rate, mid_rate, spread,
    available_amount, min_amount, max_amount,
    is_active, last_update_source, created_at, updated_at
) VALUES (
    uuid_generate_v4(),
    'eecc7d6a-276f-4ec0-a502-be3fa5c4ee3f', -- changeur test ID
    'USD', 'XOF',
    590.0, 600.0, 595.0, 10.0,
    50000, 1000, 250000,
    true, 'manual', NOW(), NOW()
);

-- Create XOF/EUR rate
INSERT INTO exchange_rates (
    id, changeur_id, from_currency, to_currency,
    buy_rate, sell_rate, mid_rate, spread,
    available_amount, min_amount, max_amount,
    is_active, last_update_source, created_at, updated_at
) VALUES (
    uuid_generate_v4(),
    'eecc7d6a-276f-4ec0-a502-be3fa5c4ee3f',
    'XOF', 'EUR',
    0.00151, 0.00154, 0.001525, 0.00003,
    10000, 100, 50000,
    true, 'manual', NOW(), NOW()
);

-- Create XOF/USD rate
INSERT INTO exchange_rates (
    id, changeur_id, from_currency, to_currency,
    buy_rate, sell_rate, mid_rate, spread,
    available_amount, min_amount, max_amount,
    is_active, last_update_source, created_at, updated_at
) VALUES (
    uuid_generate_v4(),
    'eecc7d6a-276f-4ec0-a502-be3fa5c4ee3f',
    'XOF', 'USD',
    0.00166, 0.00169, 0.001675, 0.00003,
    10000, 100, 50000,
    true, 'manual', NOW(), NOW()
);

-- Create GBP/XOF rate
INSERT INTO exchange_rates (
    id, changeur_id, from_currency, to_currency,
    buy_rate, sell_rate, mid_rate, spread,
    available_amount, min_amount, max_amount,
    is_active, last_update_source, created_at, updated_at
) VALUES (
    uuid_generate_v4(),
    'eecc7d6a-276f-4ec0-a502-be3fa5c4ee3f',
    'GBP', 'XOF',
    750.0, 765.0, 757.5, 15.0,
    25000, 500, 100000,
    true, 'manual', NOW(), NOW()
);

-- Create another changeur with different rates
INSERT INTO exchange_rates (
    id, changeur_id, from_currency, to_currency,
    buy_rate, sell_rate, mid_rate, spread,
    available_amount, min_amount, max_amount,
    is_active, last_update_source, created_at, updated_at
) VALUES (
    uuid_generate_v4(),
    'eecc7d6a-276f-4ec0-a502-be3fa5c4ee3f',
    'EUR', 'USD',
    1.08, 1.10, 1.09, 0.02,
    20000, 500, 100000,
    true, 'manual', NOW(), NOW()
);

-- Check the created rates
SELECT from_currency, to_currency, buy_rate, sell_rate, available_amount
FROM exchange_rates
ORDER BY created_at DESC;