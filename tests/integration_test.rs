use common::*;
use std::fs;
use std::time::Duration;
use tokio::time::sleep;

/// Basic integration test to verify error types work correctly
#[tokio::test]
async fn test_error_types() {
    let error = FileServerError::FileNotFound("test.txt".to_string());
    assert!(error.to_string().contains("File not found: test.txt"));

    let error = FileServerError::PermissionDenied("Access denied".to_string());
    assert!(error.to_string().contains("Permission denied: Access denied"));

    let error = FileServerError::InvalidPath("Bad path".to_string());
    assert!(error.to_string().contains("Invalid path: Bad path"));
}

/// Test protobuf message creation and field access
#[tokio::test]
async fn test_protobuf_messages() {
    // Test ConnectRequest
    let connect_req = ConnectRequest {
        client_id: "test-client".to_string(),
    };
    assert_eq!(connect_req.client_id, "test-client");

    // Test ConnectResponse
    let connect_resp = ConnectResponse {
        success: true,
        message: "Connected successfully".to_string(),
        available_directories: vec!["docs".to_string(), "workspace".to_string()],
    };
    assert!(connect_resp.success);
    assert_eq!(connect_resp.available_directories.len(), 2);

    // Test FileMetadata
    let metadata = FileMetadata {
        name: "test.txt".to_string(),
        size: 1024,
        is_directory: false,
        permissions: "read-write".to_string(),
        modified_time: 1234567890,
        created_time: 1234567890,
    };
    assert_eq!(metadata.name, "test.txt");
    assert_eq!(metadata.size, 1024);
    assert!(!metadata.is_directory);

    // Test FileEntry
    let entry = FileEntry {
        name: "file.txt".to_string(),
        is_directory: false,
        size: 512,
        modified_time: 1234567890,
        permissions: "read-only".to_string(),
    };
    assert_eq!(entry.name, "file.txt");
    assert_eq!(entry.size, 512);

    // Test DataChunk
    let chunk = DataChunk {
        path: "test/file.txt".to_string(),
        data: b"Hello, World!".to_vec(),
        offset: 0,
        is_last: true,
    };
    assert_eq!(chunk.data, b"Hello, World!");
    assert!(chunk.is_last);

    // Test WriteResponse
    let write_resp = WriteResponse {
        success: true,
        message: "Written successfully".to_string(),
        bytes_written: 13,
    };
    assert!(write_resp.success);
    assert_eq!(write_resp.bytes_written, 13);

    // Test DeleteRequest and DeleteResponse
    let delete_req = DeleteRequest {
        path: "test/file.txt".to_string(),
    };
    assert_eq!(delete_req.path, "test/file.txt");

    let delete_resp = DeleteResponse {
        success: true,
        message: "Deleted successfully".to_string(),
    };
    assert!(delete_resp.success);
}

/// Test that we can create and manipulate temporary files for testing
#[tokio::test]
async fn test_file_operations() {
    let temp_dir = std::env::temp_dir().join("fileserver_integration_test");
    
    // Clean up any existing test directory
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).unwrap();
    }
    
    // Create test directory
    fs::create_dir_all(&temp_dir).unwrap();
    
    // Create a test file
    let test_file = temp_dir.join("test.txt");
    fs::write(&test_file, "Integration test content").unwrap();
    
    // Verify file exists and has correct content
    assert!(test_file.exists());
    let content = fs::read_to_string(&test_file).unwrap();
    assert_eq!(content, "Integration test content");
    
    // Create a subdirectory
    let sub_dir = temp_dir.join("subdir");
    fs::create_dir_all(&sub_dir).unwrap();
    assert!(sub_dir.exists());
    assert!(sub_dir.is_dir());
    
    // Create a file in subdirectory
    let sub_file = sub_dir.join("nested.txt");
    fs::write(&sub_file, "Nested content").unwrap();
    assert!(sub_file.exists());
    
    // List directory contents
    let entries: Vec<_> = fs::read_dir(&temp_dir).unwrap().collect();
    assert_eq!(entries.len(), 2); // test.txt and subdir
    
    // Clean up
    fs::remove_dir_all(&temp_dir).unwrap();
    assert!(!temp_dir.exists());
}

/// Test async operations work correctly
#[tokio::test]
async fn test_async_operations() {
    let start = std::time::Instant::now();
    
    // Test that we can await async operations
    sleep(Duration::from_millis(10)).await;
    
    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_millis(10));
    assert!(elapsed < Duration::from_millis(100)); // Reasonable upper bound
}

/// Test error conversion and propagation
#[tokio::test]
async fn test_error_conversion() {
    // Test that std::io::Error converts to FileServerError
    let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
    let fs_error: FileServerError = io_error.into();
    assert!(fs_error.to_string().contains("IO error"));

    // Test that TOML errors convert
    let invalid_toml = "invalid toml content [[[";
    let toml_result: Result<toml::Value, toml::de::Error> = toml::from_str(invalid_toml);
    assert!(toml_result.is_err());
    
    let toml_error = toml_result.unwrap_err();
    let fs_error: FileServerError = toml_error.into();
    assert!(fs_error.to_string().contains("TOML parsing error"));
}

/// Test that Result type alias works correctly
#[tokio::test]
async fn test_result_type_alias() {
    fn test_function() -> common::Result<String> {
        Ok("Success".to_string())
    }

    fn test_error_function() -> common::Result<String> {
        Err(FileServerError::InvalidPath("Test error".to_string()))
    }

    let success_result = test_function();
    assert!(success_result.is_ok());
    assert_eq!(success_result.unwrap(), "Success");

    let error_result = test_error_function();
    assert!(error_result.is_err());
    let error = error_result.unwrap_err();
    assert!(error.to_string().contains("Invalid path: Test error"));
}