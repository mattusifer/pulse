use lettre::smtp::{
    authentication::{Credentials, Mechanism},
    extension::ClientId,
};
use lettre::{SmtpClient, Transport};
use lettre_email::Email;

use crate::config::{BroadcastConfig, Config};
use crate::error::Result;

pub struct Broadcast {
    config: BroadcastConfig,
}

#[derive(Debug)]
pub enum BroadcastMessage {
    Email { subject: String, body: String },
}

impl Broadcast {
    pub fn new() -> Result<Self> {
        Ok(Self {
            config: Config::from_file()?.broadcast,
        })
    }

    pub fn broadcast(&self, message: BroadcastMessage) -> Result<()> {
        match message {
            BroadcastMessage::Email { subject, body } => {
                self.email(subject, body)
            }
        }
    }

    fn email<S: Into<String>, T: Into<String>>(
        &self,
        subject: S,
        body: T,
    ) -> Result<()> {
        let mut email = Email::builder();
        for recipient in &self.config.email.recipients {
            email = email.to(recipient.clone())
        }

        let email = email
            .from(self.config.email.username.clone())
            .subject(subject)
            .text(body)
            .build()
            .unwrap();

        let mut mailer = SmtpClient::new_simple(&self.config.email.smtp_host)
            .unwrap()
            // Set the name sent during EHLO/HELO, default is `localhost`
            .hello_name(ClientId::Domain("my.hostname.tld".to_string()))
            // Add credentials for authentication
            .credentials(Credentials::new(
                self.config.email.username.clone(),
                self.config.email.password.clone(),
            ))
            // Enable SMTPUTF8 if the server supports it
            .smtp_utf8(true)
            // Configure expected authentication mechanism
            .authentication_mechanism(Mechanism::Plain)
            .transport();

        // Send the email
        mailer.send(email.into()).map(|_| ()).map_err(|e| {
            println!("{:?}", e);
            Into::into(e)
        })
    }
}
