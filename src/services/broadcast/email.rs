use downcast_rs::{impl_downcast, Downcast};
use lettre::smtp::{
    authentication::{Credentials, Mechanism},
    extension::ClientId,
};
use lettre::{SmtpClient, Transport};
use lettre_email::Email;

use crate::{config::EmailConfig, error::Result};

pub trait SendEmail: Downcast {
    fn email(&self, subject: String, body: String) -> Result<()>;
}

impl_downcast!(SendEmail);

pub struct Emailer {
    config: EmailConfig,
}
impl Emailer {
    pub fn new(config: EmailConfig) -> Box<dyn SendEmail>
    where
        Self: SendEmail,
    {
        Box::new(Self { config })
    }
}

impl SendEmail for Emailer {
    fn email(&self, subject: String, body: String) -> Result<()> {
        let mut email = Email::builder();
        for recipient in &self.config.recipients {
            email = email.to(recipient.clone())
        }

        let email = email
            .from(self.config.username.clone())
            .subject(subject)
            .html(body)
            .build()
            .unwrap();

        let mut mailer = SmtpClient::new_simple(&self.config.smtp_host)?
            // Add credentials for authentication
            .credentials(Credentials::new(
                self.config.username.clone(),
                self.config.password.clone(),
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
}
