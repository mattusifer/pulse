use lettre::smtp::authentication::{Credentials, Mechanism};
use lettre::{SmtpClient, Transport};
use lettre_email::Email;

use crate::{config::EmailConfig, error::Result};

pub fn send_email(config: &EmailConfig, subject: String, body: String) -> Result<()> {
    let mut email = Email::builder();
    for recipient in &config.recipients {
        email = email.to(recipient.clone())
    }

    let email = email
        .from(config.username.clone())
        .subject(subject)
        .html(body)
        .build()
        .unwrap();

    let mut mailer = SmtpClient::new_simple(&config.smtp_host)?
        // Add credentials for authentication
        .credentials(Credentials::new(
            config.username.clone(),
            config.password.clone(),
        ))
        // Enable SMTPUTF8 if the server supports it
        .smtp_utf8(true)
        // Configure expected authentication mechanism
        .authentication_mechanism(Mechanism::Plain)
        .transport();

    // Send the email
    mailer.send(email.into()).map(|_| ()).map_err(|e| {
        eprintln!("Error sending email: {:?}", e);
        Into::into(e)
    })
}
