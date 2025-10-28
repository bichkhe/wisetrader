# MySQL Connection Troubleshooting

## Issue
Cannot connect to MySQL container with password `wisetrader2025`

## Current Configuration

```yaml
# docker-compose.yml
db:
  environment:
    MYSQL_ROOT_PASSWORD: root2025
    MYSQL_DATABASE: wisetrader_db
    MYSQL_USER: wisetrader
    MYSQL_PASSWORD: wisetrader2025
```

## Possible Causes

1. **Init script runs before user is fully configured**
2. **Password not properly set**
3. **Platform compatibility issue** (linux/amd64 vs arm64)

## Solutions

### Option 1: Use Root User (Quick Fix)
```bash
docker exec wisetrader_db mysql -u root -proot2025 wisetrader_db -e "SHOW TABLES;"
```

### Option 2: Update DATABASE_URL
Change default in `shared/src/config.rs`:
```rust
database_url: std::env::var("DATABASE_URL")
    .unwrap_or_else(|_| "mysql://root:root2025@localhost:3306/wisetrader_db".to_string()),
```

### Option 3: Wait for Initialization
```bash
# Wait for MySQL to fully initialize
sleep 20
docker exec wisetrader_db mysql -u root -proot2025 -e "SELECT 1;"
```

### Option 4: Recreate with --remove-orphans
```bash
docker-compose down --remove-orphans
docker-compose up -d db
```

## Test Commands

```bash
# Test root connection
docker exec wisetrader_db mysql -u root -proot2025 wisetrader_db -e "SHOW TABLES;"

# Test wisetrader user
docker exec wisetrader_db mysql -u wisetrader -pwisetrader2025 wisetrader_db -e "SHOW TABLES;"

# Check users
docker exec wisetrader_db mysql -u root -proot2025 -e "SELECT User, Host FROM mysql.user;"
```

## Recommendation

Use root user for now until user permissions are fixed:
- Update `.env.example` with: `DATABASE_URL=mysql://root:root2025@localhost:3306/wisetrader_db`
- Or wait for proper initialization of wisetrader user

