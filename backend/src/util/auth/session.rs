use color_eyre::Result;
use uuid::Uuid;

const USER_ID_KEY: &str = "MANIFOLD__USER_ID";
const IDENTITY_KEY: &str = "MANIFOLD__IDENTITY";

#[tracing::instrument(skip(session))]
pub fn create_session_for_user(
    user_id: Uuid,
    identifier: String,
    session: &actix_session::Session,
) -> Result<()> {
    tracing::debug!("Generating session key for user with id '{}'", user_id);

    session.renew();
    session.insert(USER_ID_KEY, user_id)?;
    session.insert(IDENTITY_KEY, identifier)?;

    Ok(())
}

pub fn get_user_id_from_session(session: &actix_session::Session) -> Result<Option<Uuid>> {
    let user_id = session.get::<Uuid>(USER_ID_KEY)?;

    Ok(user_id)
}
