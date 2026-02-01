use argon2::{Argon2, PasswordHasher};
use argon2::password_hash::SaltString;
use rand::thread_rng;

fn main() {
    let password = b"password123";
    let salt = SaltString::generate(&mut thread_rng());
    let argon2 = Argon2::default();
    let password_hash = argon2.hash_password(password, &salt).unwrap();
    println!("Password hash for 'password123':");
    println!("{}", password_hash);
}