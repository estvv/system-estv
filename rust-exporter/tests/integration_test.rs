#[tokio::test]
async fn test_health_endpoint() {
    use axum_test::TestServer;
    
    let app = rust_exporter::create_app();
    let server = TestServer::new(app).unwrap();
    
    let response = server.get("/health").await;
    
    assert!(response.status_code().is_success());
}