use std::io::Write;
use std::process::{Command, Stdio};

use crate::{errors::ServiceError, models::Invitation, BIND_PORT};

/// Send an invitation using the system `ssmtp` binary.
///
/// This implementation builds a minimal RFC-822 message with HTML body
/// and writes it to the stdin of the `ssmtp` process. It mirrors the
/// behaviour of `src/email_service.rs` but sends mail via `ssmtp`.
pub fn send_invitation(invitation: &Invitation) -> Result<(), ServiceError> {
    let sending_email = std::env::var("SENDING_EMAIL_ADDRESS")
        .expect("SENDING_EMAIL_ADDRESS must be set");

    let subject = "You have been invited to join Simple-Auth-Server Rust";

    let email_body = format!(
        "Please click on the link below to complete registration. <br/>
         <a href=\"http://localhost:{BIND_PORT}/register.html?id={}&email={}\">
         http://localhost:{BIND_PORT}/register</a> <br>
         your Invitation expires on <strong>{}</strong>",
        invitation.id,
        invitation.email,
        invitation.expires_at.format("%I:%M %p %A, %-d %B, %C%y")
    );

    // Build a simple RFC-822 message with HTML content
    let message = format!(
        "From: {}\nTo: {}\nSubject: {}\nMIME-Version: 1.0\nContent-Type: text/html; charset=\"utf-8\"\n\n{}",
        sending_email, invitation.email, subject, email_body
    );

    // Spawn `ssmtp` and write the message to its stdin. The `ssmtp` binary
    // typically accepts recipient addresses on the command line.
    let mut child = match Command::new("ssmtp")
        .arg(&invitation.email)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(err) => {
            println!("Failed to spawn ssmtp: {err:#?}");
            return Err(ServiceError::InternalServerError);
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        if let Err(err) = stdin.write_all(message.as_bytes()) {
            println!("Failed to write to ssmtp stdin: {err:#?}");
            return Err(ServiceError::InternalServerError);
        }
    }

    // Wait for ssmtp to finish and check its exit status
    match child.wait_with_output() {
        Ok(output) => {
            if output.status.success() {
                println!("ssmtp sent email to {}", &invitation.email);
                Ok(())
            } else {
                println!(
                    "ssmtp failed. status={:?} stdout={:?} stderr={:?}",
                    output.status, String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr)
                );
                Err(ServiceError::InternalServerError)
            }
        }
        Err(err) => {
            println!("Failed to wait for ssmtp: {err:#?}");
            Err(ServiceError::InternalServerError)
        }
    }
}
