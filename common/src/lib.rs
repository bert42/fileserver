pub mod error;

tonic::include_proto!("fileserver");

pub use error::FileServerError;