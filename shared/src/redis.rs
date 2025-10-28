use anyhow::Result;
use redis::Client;

pub type Redis = Client;

pub fn get_redis_client(redis_url: &str) -> Result<Redis> {
    let client = Client::open(redis_url)?;
    Ok(client)
}
