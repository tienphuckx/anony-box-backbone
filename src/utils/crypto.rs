use rand::{distributions::Alphanumeric, thread_rng, Rng};

use sha2::{Digest, Sha256};

pub fn generate_random_salt(length: usize) -> String {
  thread_rng()
    .sample_iter(&Alphanumeric)
    .take(length)
    .map(char::from)
    .collect()
}

pub fn generate_secret_code(plain: &str) -> String {
  let salt = generate_random_salt(16);
  let timestamp = chrono::Utc::now().timestamp_millis();
  let data = format!("{}{}{}", plain, timestamp, salt);

  let mut hasher = Sha256::new();

  // write input message
  hasher.update(data.as_bytes());
  let result = format!("{:X}", hasher.finalize());
  result
}
