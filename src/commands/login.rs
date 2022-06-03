use momento::{momento::auth::LoginResult, response::error::MomentoError};

pub async fn login() -> LoginResult {
    momento::momento::auth::login(|action| match action {
        momento::momento::auth::LoginAction::OpenBrowser(open) => {
            match webbrowser::open(&open.url) {
                Ok(_) => {
                    log::debug!("opened browser to {}", open.url);
                    None
                }
                Err(e) => Some(Err(MomentoError::ClientSdkError(format!(
                    "Unable to open browser: {:?}",
                    e
                )))),
            }
        }
        momento::momento::auth::LoginAction::ShowMessage(message) => {
            eprintln!("{}", message.text);
            None
        }
    })
    .await
}
