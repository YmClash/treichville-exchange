-- Create changeur profile for the existing changeur
INSERT INTO changeur_profiles (
    user_id, 
    business_name, 
    business_address, 
    rating, 
    total_transactions, 
    successful_transactions,
    average_response_time,
    daily_volume_limit,
    monthly_volume_limit,
    verification_status,
    verification_date,
    bank_account_verified,
    license_number,
    location_latitude,
    location_longitude,
    opening_hours,
    closing_hours,
    working_days,
    created_at,
    updated_at
) VALUES (
    'eecc7d6a-276f-4ec0-a502-be3fa5c4ee3f', -- changeur user_id
    'Bureau de Change Treichville',
    'Rue 12, Treichville, Abidjan',
    4.5,
    100,
    98,
    300, -- response time in seconds
    10000000.00, -- 10M XOF daily limit
    200000000.00, -- 200M XOF monthly limit
    'verified',
    NOW(),
    true,
    'BC-2024-001',
    5.3080, -- Latitude Treichville
    -4.0070, -- Longitude Treichville
    '08:00:00',
    '19:00:00',
    'monday,tuesday,wednesday,thursday,friday,saturday',
    NOW(),
    NOW()
) ON CONFLICT (user_id) DO UPDATE SET
    business_name = EXCLUDED.business_name,
    updated_at = NOW();