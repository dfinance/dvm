use crate::compiled_protos::ds_grpc::{DsRawResponse, ds_raw_response};

pub mod mvir;

impl DsRawResponse {
    pub fn with_blob(blob: &[u8]) -> DsRawResponse {
        DsRawResponse {
            blob: blob.to_vec(),
            error_code: ds_raw_response::ErrorCode::None as i32,
            error_message: "".to_string(),
        }
    }

    pub fn with_error(
        error_code: ds_raw_response::ErrorCode,
        error_message: String,
    ) -> DsRawResponse {
        DsRawResponse {
            blob: vec![],
            error_code: error_code as i32,
            error_message,
        }
    }
}
