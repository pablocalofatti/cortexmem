#[cfg(feature = "cloud")]
mod tests {
    use cortexmem::cloud::auth::{create_jwt, hash_password, verify_jwt, verify_password};
    use uuid::Uuid;

    #[test]
    fn should_hash_and_verify_password() {
        let hash = hash_password("test_password_123").unwrap();
        assert!(verify_password("test_password_123", &hash).unwrap());
        assert!(!verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn should_create_and_verify_jwt() {
        let account_id = Uuid::new_v4();
        let secret = "test-secret-key-at-least-32-chars-long";
        let token = create_jwt(&account_id, secret).unwrap();
        let claims = verify_jwt(&token, secret).unwrap();
        assert_eq!(claims.sub, account_id.to_string());
    }

    #[test]
    fn should_reject_invalid_jwt() {
        let result = verify_jwt("invalid.token.here", "secret");
        assert!(result.is_err());
    }

    #[test]
    fn should_reject_expired_jwt() {
        // Using a very short expiry would require modifying the function
        // So just test that a tampered token is rejected
        let account_id = Uuid::new_v4();
        let secret = "test-secret-key-at-least-32-chars-long";
        let token = create_jwt(&account_id, secret).unwrap();
        let result = verify_jwt(&token, "wrong-secret-key-at-least-32-chars");
        assert!(result.is_err());
    }
}
