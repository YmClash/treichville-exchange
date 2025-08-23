-- Migration pour créer les wallets initiaux pour les utilisateurs existants

-- D'abord, ajoutons une contrainte unique si elle n'existe pas
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint 
        WHERE conname = 'wallets_user_currency_unique'
    ) THEN
        ALTER TABLE wallets 
        ADD CONSTRAINT wallets_user_currency_unique 
        UNIQUE (user_id, currency);
    END IF;
END $$;

-- Fonction pour créer les wallets par défaut pour un utilisateur
CREATE OR REPLACE FUNCTION create_default_wallets(p_user_id UUID)
RETURNS void AS $$
BEGIN
    -- Créer wallet XOF (monnaie principale)
    INSERT INTO wallets (id, user_id, currency, balance, reserved_balance, created_at, updated_at)
    VALUES (
        uuid_generate_v4(),
        p_user_id,
        'XOF',
        0.00,
        0.00,
        NOW(),
        NOW()
    ) ON CONFLICT (user_id, currency) DO NOTHING;
    
    -- Créer wallet EUR
    INSERT INTO wallets (id, user_id, currency, balance, reserved_balance, created_at, updated_at)
    VALUES (
        uuid_generate_v4(),
        p_user_id,
        'EUR',
        0.00,
        0.00,
        NOW(),
        NOW()
    ) ON CONFLICT (user_id, currency) DO NOTHING;
    
    -- Créer wallet USD
    INSERT INTO wallets (id, user_id, currency, balance, reserved_balance, created_at, updated_at)
    VALUES (
        uuid_generate_v4(),
        p_user_id,
        'USD',
        0.00,
        0.00,
        NOW(),
        NOW()
    ) ON CONFLICT (user_id, currency) DO NOTHING;
END;
$$ LANGUAGE plpgsql;

-- Créer les wallets pour tous les utilisateurs existants
DO $$
DECLARE
    user_record RECORD;
BEGIN
    FOR user_record IN SELECT id FROM users
    LOOP
        PERFORM create_default_wallets(user_record.id);
    END LOOP;
END $$;

-- Trigger pour créer automatiquement les wallets pour les nouveaux utilisateurs
CREATE OR REPLACE FUNCTION auto_create_wallets()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM create_default_wallets(NEW.id);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER create_wallets_on_user_insert
    AFTER INSERT ON users
    FOR EACH ROW
    EXECUTE FUNCTION auto_create_wallets();

-- Insérer des fonds de test pour le changeur (pour permettre les échanges)
UPDATE wallets 
SET balance = 1000000.00 
WHERE user_id = (SELECT id FROM users WHERE phone = '+22507654321')
AND currency = 'XOF';

UPDATE wallets 
SET balance = 5000.00 
WHERE user_id = (SELECT id FROM users WHERE phone = '+22507654321')
AND currency = 'EUR';

UPDATE wallets 
SET balance = 3000.00 
WHERE user_id = (SELECT id FROM users WHERE phone = '+22507654321')
AND currency = 'USD';

-- Insérer des fonds de test pour le client
UPDATE wallets 
SET balance = 50000.00 
WHERE user_id = (SELECT id FROM users WHERE phone = '+22501234567')
AND currency = 'XOF';