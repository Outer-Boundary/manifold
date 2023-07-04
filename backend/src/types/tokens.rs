use pasetors::claims::Claims;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ConfirmationToken {
    pub user_id: Uuid,
    pub claims: Claims,
}
