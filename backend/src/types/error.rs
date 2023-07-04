use serde::Serialize;

type ErrorCode = u16;

// Model representing a user.
#[derive(Serialize)]
pub struct ErrorResponse {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<FieldError>>,
}

impl ErrorResponse {
    pub fn new<T>(code: ErrorCode, message: T) -> Self
    where
        T: ToString,
    {
        ErrorResponse {
            code,
            message: message.to_string(),
            description: None,
            errors: None,
        }
    }

    pub fn description<T>(&mut self, content: T) -> &mut Self
    where
        T: ToString,
    {
        self.description = Some(content.to_string());
        self
    }

    pub fn field_errors<T>(&mut self, errors: Vec<FieldError>) -> &mut Self {
        self.errors = Some(errors);
        self
    }
}

#[derive(Serialize)]
pub struct FieldError {
    pub code: ErrorCode,
    pub field: String,
    pub message: String,
}

impl FieldError {
    pub fn new<T>(code: ErrorCode, field_name: String, message: T) -> Self
    where
        T: ToString,
    {
        FieldError {
            code,
            field: field_name,
            message: message.to_string(),
        }
    }
}
