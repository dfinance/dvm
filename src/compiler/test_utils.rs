use tonic::Response;
use crate::compiled_protos::ds_grpc::{DsRawResponse, ds_raw_response};

pub fn new_response(blob: &[u8]) -> Response<DsRawResponse> {
    Response::new(DsRawResponse {
        blob: blob.to_vec(),
        error_code: ds_raw_response::ErrorCode::None as i32,
        error_message: vec![],
    })
}

#[allow(dead_code)]
pub fn new_error_response(
    error_code: ds_raw_response::ErrorCode,
    error_message: String,
) -> Response<DsRawResponse> {
    Response::new(DsRawResponse {
        blob: vec![],
        error_code: error_code as i32,
        error_message: error_message.into_bytes(),
    })
}
