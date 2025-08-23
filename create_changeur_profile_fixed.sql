-- Create changeur profile for the existing changeur
INSERT INTO changeur_profiles (
    user_id, 
    business_name, 
    business_address,
    business_phone,
    license_number,
    rating, 
    total_transactions, 
    total_volume,
    commission_rate,
    available_currencies,
    operating_hours,
    location_latitude,
    location_longitude,
    is_verified,
    verified_at,
    metadata,
    created_at,
    updated_at
) VALUES (
    'eecc7d6a-276f-4ec0-a502-be3fa5c4ee3f', -- changeur user_id
    'Bureau de Change Treichville',
    'Rue 12, Treichville, Abidjan',
    '+22507654321',
    'BC-2024-001',
    4.5,
    100,
    10000000.00, -- 10M XOF volume
    0.015, -- 1.5% commission
    ARRAY['XOF', 'EUR', 'USD', 'GBP'],
    '{"monday": {"open": "08:00", "close": "19:00"}, "tuesday": {"open": "08:00", "close": "19:00"}, "wednesday": {"open": "08:00", "close": "19:00"}, "thursday": {"open": "08:00", "close": "19:00"}, "friday": {"open": "08:00", "close": "19:00"}, "saturday": {"open": "08:00", "close": "14:00"}}',
    5.3080, -- Latitude Treichville
    -4.0070, -- Longitude Treichville
    true,
    NOW(),
    '{"daily_limit": 10000000, "monthly_limit": 200000000}',
    NOW(),
    NOW()
) ON CONFLICT (user_id) DO UPDATE SET
    business_name = EXCLUDED.business_name,
    business_address = EXCLUDED.business_address,
    rating = EXCLUDED.rating,
    is_verified = EXCLUDED.is_verified,
    updated_at = NOW();