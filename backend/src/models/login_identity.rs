use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LoginIdentityType {
    Email,
}

impl LoginIdentityType {
    pub fn all() -> Vec<LoginIdentityType> {
        vec![LoginIdentityType::Email]
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum LoginIdentity {
    Email(LIEmail),
}

impl LoginIdentity {
    pub fn get_type(&self) -> LoginIdentityType {
        match self {
            LoginIdentity::Email(_) => LoginIdentityType::Email,
        }
    }

    pub fn identifier(&self) -> String {
        match self {
            LoginIdentity::Email(li) => li.email.clone(),
        }
    }
}

// Model representing an email login identity stored in the login_identity__email table.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LIEmail {
    pub user_id: Uuid,

    pub email: String,
    pub password_hash: String,
    pub salt: String,

    pub verified: bool,

    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// Enum representing all possible login identities that a user can use when authenticating or creating a new account.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum ClientLoginIdentity {
    Email(ClientLIEmail),
}

impl ClientLoginIdentity {
    pub fn get_type(&self) -> LoginIdentityType {
        match self {
            ClientLoginIdentity::Email(_) => LoginIdentityType::Email,
        }
    }

    pub fn identifier(&self) -> String {
        match self {
            ClientLoginIdentity::Email(li) => li.email.clone(),
        }
    }
}

// Model representing the data sent from the client to log in or to create a new user.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ClientLIEmail {
    pub email: String,
    pub password: String,
}
