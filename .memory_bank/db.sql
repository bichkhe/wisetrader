CREATE TABLE users (
  id BIGINT PRIMARY KEY, -- telegram user id
  username TEXT,
  language TEXT,
  created_at TIMESTAMP DEFAULT now(),
  subscription_tier TEXT,
  subscription_expires TIMESTAMP,
  live_trading_enabled BOOLEAN DEFAULT FALSE
);

CREATE TABLE strategies (
  id SERIAL PRIMARY KEY,
  name TEXT,
  description TEXT,
  repo_ref TEXT,
  created_at TIMESTAMP DEFAULT now()
);

CREATE TABLE user_strategies (
  id SERIAL PRIMARY KEY,
  user_id BIGINT REFERENCES users(id),
  strategy_id INT REFERENCES strategies(id),
  params JSONB,
  active BOOLEAN DEFAULT true
);

CREATE TABLE signals (
  id BIGSERIAL PRIMARY KEY,
  strategy_id INT,
  payload JSONB,
  sent_at TIMESTAMP DEFAULT now()
);

CREATE TABLE orders (
  id BIGSERIAL PRIMARY KEY,
  user_id BIGINT,
  exchange TEXT,
  symbol TEXT,
  side TEXT,
  qty NUMERIC,
  price NUMERIC,
  status TEXT,
  external_id TEXT,
  created_at TIMESTAMP DEFAULT now()
);
