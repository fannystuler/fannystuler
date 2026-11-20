use zapret2_gui::lists::{normalize_domain, HostlistError, HostlistManager};

#[test]
fn normalize_plain_domain() {
    assert_eq!(normalize_domain("example.com").unwrap(), Some("example.com".to_string()));
}

#[test]
fn normalize_with_scheme_and_path() {
    assert_eq!(
        normalize_domain("https://Example.COM/path?q=1").unwrap(),
        Some("example.com".to_string())
    );
}

#[test]
fn normalize_with_port() {
    assert_eq!(
        normalize_domain("example.com:443").unwrap(),
        Some("example.com".to_string())
    );
}

#[test]
fn normalize_leading_dot() {
    assert_eq!(normalize_domain(".example.com").unwrap(), Some("example.com".to_string()));
}

#[test]
fn normalize_wildcard() {
    assert_eq!(normalize_domain("*.example.com").unwrap(), Some("*.example.com".to_string()));
}

#[test]
fn normalize_empty_and_comments() {
    assert_eq!(normalize_domain("").unwrap(), None);
    assert_eq!(normalize_domain("   ").unwrap(), None);
    assert_eq!(normalize_domain("# comment").unwrap(), None);
    assert_eq!(normalize_domain("; comment").unwrap(), None);
}

#[test]
fn normalize_invalid_domain() {
    assert!(matches!(normalize_domain("bad domain"), Err(HostlistError::InvalidDomain(_))));
    assert!(matches!(normalize_domain("-bad.com"), Err(HostlistError::InvalidDomain(_))));
    assert!(matches!(normalize_domain("bad-.com"), Err(HostlistError::InvalidDomain(_))));
}

#[test]
fn manager_add_and_dedup() {
    let mut m = HostlistManager::new();
    assert!(m.add_domain("example.com").unwrap());
    assert!(!m.add_domain("example.com").unwrap());
    assert!(!m.add_domain("EXAMPLE.com").unwrap());
    assert_eq!(m.domains, vec!["example.com".to_string()]);
}

#[test]
fn manager_remove() {
    let mut m = HostlistManager::new();
    m.add_domain("a.com").unwrap();
    m.add_domain("b.com").unwrap();
    assert!(m.remove_domain("A.COM"));
    assert!(!m.remove_domain("a.com"));
    assert_eq!(m.domains, vec!["b.com".to_string()]);
}

#[test]
fn manager_deduplicate_and_sort() {
    let mut m = HostlistManager::new();
    m.domains = vec!["b.com".into(), "a.com".into(), "b.com".into()];
    assert_eq!(m.deduplicate(), 1);
    m.sort_alphabetically();
    assert_eq!(m.domains, vec!["a.com".to_string(), "b.com".to_string()]);
}

#[test]
fn manager_save_and_load_roundtrip() {
    let dir = std::env::temp_dir().join(format!("zapret2_test_{}", std::process::id()));
    let path = dir.join("list.txt");
    let mut m = HostlistManager::new();
    m.add_domain("example.com").unwrap();
    m.add_domain("test.org").unwrap();
    m.save_to_file(&path).unwrap();
    let loaded = HostlistManager::load_from_file(&path).unwrap();
    assert_eq!(loaded.domains, m.domains);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn manager_load_missing_file_is_empty() {
    let path = std::env::temp_dir().join("zapret2_definitely_missing_file_xyz.txt");
    let _ = std::fs::remove_file(&path);
    let m = HostlistManager::load_from_file(&path).unwrap();
    assert!(m.domains.is_empty());
}
