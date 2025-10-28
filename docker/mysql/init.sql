-- Create database schema
CREATE TABLE users (
  id BIGINT PRIMARY KEY,
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
  params JSON,
  active BOOLEAN DEFAULT true
);

CREATE TABLE signals (
  id BIGSERIAL PRIMARY KEY,
  strategy_id INT,
  payload JSON,
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

CREATE TABLE billing_plans (
  id VARCHAR(50) PRIMARY KEY,
  name VARCHAR(100) NOT NULL,
  price_monthly_usd DECIMAL(10, 2) NOT NULL,
  duration_days INT,
  features JSON,
  created_at TIMESTAMP DEFAULT now()
);

CREATE TABLE invoices (
  id BIGSERIAL PRIMARY KEY,
  user_id BIGINT REFERENCES users(id),
  plan_id VARCHAR(50) REFERENCES billing_plans(id),
  amount DECIMAL(10, 2),
  currency VARCHAR(10) DEFAULT 'USD',
  status VARCHAR(20),
  external_payment_id VARCHAR(255),
  created_at TIMESTAMP DEFAULT now()
);

CREATE TABLE payment_transactions (
  id BIGSERIAL PRIMARY KEY,
  invoice_id BIGINT REFERENCES invoices(id),
  transaction_type VARCHAR(20),
  amount DECIMAL(10, 2),
  status VARCHAR(20),
  external_id VARCHAR(255),
  metadata JSON,
  created_at TIMESTAMP DEFAULT now()
);

-- Seed billing plans
INSERT INTO billing_plans (id, name, price_monthly_usd, duration_days, features) VALUES
('free_trial', 'Free Trial', 0, 7, '["signals (delayed)", "1 backtest job"]'),
('basic', 'Basic', 29, NULL, '["signals (real-time)", "1 strategy", "no live trading"]'),
('pro', 'Pro', 99, NULL, '["live trading", "3 strategies", "priority support"]');

-- Seed initial strategies
INSERT INTO strategies (id, name, description, repo_ref) VALUES
(1, 'Moving Average Crossover', 'Buy when fast MA crosses above slow MA', 'ma_crossover'),
(2, 'RSI Divergence', 'Buy on RSI oversold with bullish divergence', 'rsi_divergence'),
(3, 'Bollinger Bands Squeeze', 'Buy when volatility increases from squeeze', 'bb_squeeze');

-- Convert strategies.id to AUTO_INCREMENT
ALTER TABLE strategies MODIFY id INT AUTO_INCREMENT;
ALTER TABLE user_strategies MODIFY id INT AUTO_INCREMENT;
ALTER TABLE signals MODIFY id BIGINT AUTO_INCREMENT;
ALTER TABLE orders MODIFY id BIGINT AUTO_INCREMENT;
ALTER TABLE invoices MODIFY id BIGINT AUTO_INCREMENT;
ALTER TABLE payment_transactions MODIFY id BIGINT AUTO_INCREMENT;

