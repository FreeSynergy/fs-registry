//! Integration tests for fs-registry.

use fs_registry::{Registry, RegistryError, ServiceEntry, ServiceStatus};

// ── F5: endpoint_for_capability ───────────────────────────────────────────────

#[tokio::test]
async fn endpoint_for_capability_returns_up_service() {
    let reg = registry().await;
    reg.register(ServiceEntry::new("kanidm", "iam", "http://kanidm:8443"))
        .await
        .unwrap();

    let ep = reg.endpoint_for_capability("iam").await.unwrap();
    assert_eq!(ep, Some("http://kanidm:8443".to_string()));
}

#[tokio::test]
async fn endpoint_for_capability_skips_down_service() {
    let reg = registry().await;
    let entry = ServiceEntry::new("kanidm", "iam", "http://kanidm:8443");
    let id = entry.id.clone();
    reg.register(entry).await.unwrap();
    reg.set_status(&id, ServiceStatus::Down).await.unwrap();

    let ep = reg.endpoint_for_capability("iam").await.unwrap();
    assert_eq!(ep, None, "down service should not be returned");
}

#[tokio::test]
async fn endpoint_for_capability_returns_none_when_empty() {
    let reg = registry().await;
    let ep = reg.endpoint_for_capability("iam").await.unwrap();
    assert_eq!(ep, None);
}

#[tokio::test]
async fn endpoint_for_capability_prefers_up_over_down() {
    let reg = registry().await;
    // Register kanidm as down
    let e1 = ServiceEntry::new("kanidm", "iam", "http://kanidm:8443");
    let id1 = e1.id.clone();
    reg.register(e1).await.unwrap();
    reg.set_status(&id1, ServiceStatus::Down).await.unwrap();
    // Register a second IAM provider that is up
    reg.register(ServiceEntry::new(
        "authentik",
        "iam",
        "http://authentik:9000",
    ))
    .await
    .unwrap();

    let ep = reg.endpoint_for_capability("iam").await.unwrap();
    assert_eq!(ep, Some("http://authentik:9000".to_string()));
}

async fn registry() -> Registry {
    Registry::open(":memory:").await.expect("open failed")
}

#[tokio::test]
async fn register_and_query_by_capability() {
    let reg = registry().await;
    let entry = ServiceEntry::new("kanidm", "iam", "http://kanidm:8443");

    reg.register(entry).await.unwrap();

    let results = reg.by_capability("iam").await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].service_id, "kanidm");
    assert_eq!(results[0].endpoint, "http://kanidm:8443");
    assert!(results[0].is_up());
}

#[tokio::test]
async fn one_service_multiple_capabilities() {
    let reg = registry().await;
    reg.register(ServiceEntry::new("kanidm", "iam", "http://kanidm:8443"))
        .await
        .unwrap();
    reg.register(ServiceEntry::new("kanidm", "scim", "http://kanidm:8443"))
        .await
        .unwrap();

    let all = reg.by_service("kanidm").await.unwrap();
    assert_eq!(all.len(), 2);

    let iam = reg.by_capability("iam").await.unwrap();
    let scim = reg.by_capability("scim").await.unwrap();
    assert_eq!(iam.len(), 1);
    assert_eq!(scim.len(), 1);
}

#[tokio::test]
async fn re_register_updates_endpoint() {
    let reg = registry().await;
    reg.register(ServiceEntry::new("stalwart", "mail", "http://stalwart:25"))
        .await
        .unwrap();
    reg.register(ServiceEntry::new("stalwart", "mail", "http://stalwart:587"))
        .await
        .unwrap();

    let results = reg.by_capability("mail").await.unwrap();
    assert_eq!(results.len(), 1, "should not create duplicate");
    assert_eq!(results[0].endpoint, "http://stalwart:587");
}

#[tokio::test]
async fn deregister_removes_all_service_entries() {
    let reg = registry().await;
    reg.register(ServiceEntry::new("kanidm", "iam", "http://kanidm:8443"))
        .await
        .unwrap();
    reg.register(ServiceEntry::new("kanidm", "scim", "http://kanidm:8443"))
        .await
        .unwrap();

    reg.deregister("kanidm").await.unwrap();

    let all = reg.all().await.unwrap();
    assert!(all.is_empty());
}

#[tokio::test]
async fn set_status_marks_service_down() {
    let reg = registry().await;
    let entry = ServiceEntry::new("kanidm", "iam", "http://kanidm:8443");
    let id = entry.id.clone();
    reg.register(entry).await.unwrap();

    reg.set_status(&id, ServiceStatus::Down).await.unwrap();

    let results = reg.by_capability("iam").await.unwrap();
    assert_eq!(results[0].status, ServiceStatus::Down);
    assert!(!results[0].is_up());
}

#[tokio::test]
async fn set_status_not_found_returns_error() {
    let reg = registry().await;
    let result = reg
        .set_status("nonexistent::cap", ServiceStatus::Down)
        .await;
    assert!(matches!(result, Err(RegistryError::NotFound { .. })));
}
