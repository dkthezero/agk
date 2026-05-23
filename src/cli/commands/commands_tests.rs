use std::collections::HashSet;

#[test]
fn profile_session_keys_are_unique_for_rapid_calls() {
    let mut keys = HashSet::new();
    for _ in 0..256 {
        let key = super::generate_profile_session_key();
        assert!(keys.insert(key));
    }
}
