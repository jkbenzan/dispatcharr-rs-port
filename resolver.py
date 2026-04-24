with open("src/auth.rs", "r") as f:
    auth_rs = f.read()

resolved_auth_rs = auth_rs.replace("""<<<<<<< HEAD
    fn test_generate_jwt_normal_user() {
        let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());

        let mock_user = user::Model {
            id: 43,
            password: "hashed_password".to_string(),
            last_login: Some(now.clone()),
            is_superuser: false,
            username: "normaluser".to_string(),
            first_name: "Normal".to_string(),
            last_name: "User".to_string(),
            email: "normal@example.com".to_string(),
            is_staff: false,
=======
    fn test_generate_jwt_invalid_secret() {
        let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());

        let mock_user = user::Model {
            id: 42,
            password: "hashed_password".to_string(),
            last_login: Some(now.clone()),
            is_superuser: true,
            username: "testuser".to_string(),
            first_name: "Test".to_string(),
            last_name: "User".to_string(),
            email: "test@example.com".to_string(),
            is_staff: true,
>>>>>>> origin/main
            is_active: true,
            date_joined: now,
            avatar_config: None,
            user_level: 1,
            custom_properties: None,
            api_key: None,
            stream_limit: 10,
        };

<<<<<<< HEAD
        let token_result = generate_jwt(&mock_user);
        assert!(token_result.is_ok(), "JWT generation should succeed");
        let token = token_result.unwrap();

        // Verify the token can be decoded correctly
        let mut validation = Validation::default();
        validation.validate_exp = false; // We can check exp manually

        let token_data =
            decode::<Claims>(&token, &DecodingKey::from_secret(JWT_SECRET), &validation)
                .expect("Failed to decode the generated JWT");

        assert_eq!(token_data.claims.user_id, 43);
        assert_eq!(token_data.claims.username, "normaluser");
        assert_eq!(token_data.claims.is_superuser, false);

        let current_time = chrono::Utc::now().timestamp() as usize;
        assert!(
            token_data.claims.exp > current_time,
            "Expiration time should be in the future"
        );
        assert!(
            token_data.claims.exp <= current_time + JWT_EXPIRATION_SECS + 5,
            "Expiration time should be roughly now + JWT_EXPIRATION_SECS"
        );
=======
        let token = generate_jwt(&mock_user).expect("JWT generation should succeed");

        let mut validation = Validation::default();
        validation.validate_exp = false;

        let result = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(b"wrong_secret"),
            &validation,
        );

        assert!(result.is_err(), "JWT decoding should fail with an incorrect secret");
>>>>>>> origin/main""", """    fn test_generate_jwt_normal_user() {
        let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());

        let mock_user = user::Model {
            id: 43,
            password: "hashed_password".to_string(),
            last_login: Some(now.clone()),
            is_superuser: false,
            username: "normaluser".to_string(),
            first_name: "Normal".to_string(),
            last_name: "User".to_string(),
            email: "normal@example.com".to_string(),
            is_staff: false,
            is_active: true,
            date_joined: now,
            avatar_config: None,
            user_level: 1,
            custom_properties: None,
            api_key: None,
            stream_limit: 10,
        };

        let token_result = generate_jwt(&mock_user);
        assert!(token_result.is_ok(), "JWT generation should succeed");
        let token = token_result.unwrap();

        // Verify the token can be decoded correctly
        let mut validation = Validation::default();
        validation.validate_exp = false; // We can check exp manually

        let token_data =
            decode::<Claims>(&token, &DecodingKey::from_secret(JWT_SECRET), &validation)
                .expect("Failed to decode the generated JWT");

        assert_eq!(token_data.claims.user_id, 43);
        assert_eq!(token_data.claims.username, "normaluser");
        assert_eq!(token_data.claims.is_superuser, false);

        let current_time = chrono::Utc::now().timestamp() as usize;
        assert!(
            token_data.claims.exp > current_time,
            "Expiration time should be in the future"
        );
        assert!(
            token_data.claims.exp <= current_time + JWT_EXPIRATION_SECS + 5,
            "Expiration time should be roughly now + JWT_EXPIRATION_SECS"
        );
    }

    #[test]
    fn test_generate_jwt_invalid_secret() {
        let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());

        let mock_user = user::Model {
            id: 42,
            password: "hashed_password".to_string(),
            last_login: Some(now.clone()),
            is_superuser: true,
            username: "testuser".to_string(),
            first_name: "Test".to_string(),
            last_name: "User".to_string(),
            email: "test@example.com".to_string(),
            is_staff: true,
            is_active: true,
            date_joined: now,
            avatar_config: None,
            user_level: 1,
            custom_properties: None,
            api_key: None,
            stream_limit: 10,
        };

        let token = generate_jwt(&mock_user).expect("JWT generation should succeed");

        let mut validation = Validation::default();
        validation.validate_exp = false;

        let result = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(b"wrong_secret"),
            &validation,
        );

        assert!(result.is_err(), "JWT decoding should fail with an incorrect secret");""")

with open("src/auth.rs", "w") as f:
    f.write(resolved_auth_rs)
