use crate::{models::login_identity::LoginIdentityType, types::redis::RedisPool};
use color_eyre::{eyre::eyre, Result};
use lettre::{
    message::{header::ContentType, Mailbox, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
};
use uuid::Uuid;

use super::{auth::tokens::ConfirmationTokenBuilder, configuration::get_config};

#[tracing::instrument(skip(html_content, text_content),fields(subject = %subject.clone().into()))]
pub async fn send_email(
    sender_email: Option<String>,
    recipient_email: String,
    recipient_username: String,
    subject: impl Into<String> + std::clone::Clone,
    html_content: impl Into<String>,
    text_content: impl Into<String>,
) -> Result<()> {
    let config = get_config()?;

    let email = lettre::Message::builder()
        .from(Mailbox::new(
            Some("Manifold".into()),
            sender_email
                .unwrap_or(config.email.host_user.clone())
                .parse()?,
        ))
        .to(Mailbox::new(
            Some(recipient_username),
            recipient_email.parse()?,
        ))
        .subject(subject.clone())
        .multipart(
            MultiPart::alternative()
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_PLAIN)
                        .body(text_content.into()),
                )
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_HTML)
                        .body(html_content.into()),
                ),
        )?;

    let credentials = Credentials::new(config.email.host_user, config.email.host_user_password);

    let mailer: AsyncSmtpTransport<Tokio1Executor> =
        AsyncSmtpTransport::<Tokio1Executor>::relay(&config.email.host)?
            .credentials(credentials)
            .build();

    match mailer.send(email).await {
        Ok(_) => {
            tracing::debug!(
                "Successfully sent email with subject '{}' to recipient '{}'",
                subject.clone().into(),
                recipient_email
            );
            Ok(())
        }
        Err(err) => {
            tracing::error!(
                "Unable to send email with subject '{}' to recipient '{}': {}",
                subject.clone().into(),
                recipient_email,
                err
            );
            Err(eyre!(
                "Unable to send email with subject '{}' to recipient '{}': {}",
                subject.clone().into(),
                recipient_email,
                err
            ))
        }
    }
}

#[tracing::instrument(skip(redis))]
pub async fn send_multipart_email(
    subject: String,
    user_id: Uuid,
    recipient_email: String,
    recipient_username: String,
    template_name: &str,
    li_type: LoginIdentityType,
    redis: &RedisPool,
) -> Result<()> {
    let config = get_config()?;

    let (issued_token, token_ttl) = ConfirmationTokenBuilder::create(user_id, redis)?
        .add_claim("li_type".to_string(), li_type)?
        .issue_confirmation_token()
        .await?;

    let confirmation_link = if template_name == "password_reset_email.html" {
        format!(
            "{}/change-password?token={}",
            config.client_url, issued_token
        )
    } else {
        format!("{}/verify?token={}", config.client_url, issued_token)
    };

    let current_date_time = chrono::Utc::now();
    let dt = current_date_time + chrono::Duration::minutes(token_ttl);

    let template = crate::ENV.get_template(template_name)?;
    let ctx = minijinja::context! {
        title => &subject,
        confirmation_link => &confirmation_link,
        domain => &config.client_url,
        expiration_time => &token_ttl,
        exact_time => &dt.format("%A %B %d, %Y at %r").to_string()
    };
    let html_text = template.render(ctx)?;
    let text = format!(
        r#"
        Click the link below to confirm your email address.
        {}
        "#,
        confirmation_link
    );

    actix_web::rt::spawn(send_email(
        None,
        recipient_email,
        recipient_username,
        subject,
        html_text,
        text,
    ));

    Ok(())
}
