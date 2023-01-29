use crate::utils::console::console_info;
use momento::auth::{AuthError, Credentials, EarlyOutActionResult, LoginAction};
use momento::MomentoError;
use qrcode::render::unicode;
use qrcode::QrCode;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
pub enum LoginMode {
    Browser,
    Qr,
}

pub async fn login(login_mode: LoginMode) -> Result<Credentials, AuthError> {
    momento::auth::login(match login_mode {
        LoginMode::Browser => login_with_browser,
        LoginMode::Qr => login_with_qr_code,
    })
    .await
}

fn login_with_browser(action: LoginAction) -> EarlyOutActionResult {
    match action {
        momento::auth::LoginAction::OpenBrowser(open) => match webbrowser::open(&open.url) {
            Ok(_) => {
                log::debug!("opened browser to {}", open.url);
                None
            }
            Err(e) => Some(Err(MomentoError::ClientSdkError(format!(
                "Unable to open browser: {e:?}"
            )))),
        },
        momento::auth::LoginAction::ShowMessage(message) => {
            console_info!("{}", message.text);
            None
        }
    }
}

fn login_with_qr_code(action: LoginAction) -> EarlyOutActionResult {
    match action {
        momento::auth::LoginAction::OpenBrowser(open) => {
            console_info!("Navigate here to log in: {}", open.url);
            match QrCode::new(open.url) {
                Ok(code) => {
                    let image = code
                        .render::<unicode::Dense1x2>()
                        .dark_color(unicode::Dense1x2::Dark)
                        .light_color(unicode::Dense1x2::Light)
                        .build();
                    console_info!("{}", image);
                    None
                }
                Err(e) => Some(Err(MomentoError::ClientSdkError(format!(
                    "Unable to generate qr code: {e:?}"
                )))),
            }
        }
        momento::auth::LoginAction::ShowMessage(message) => {
            console_info!("{}", message.text);
            None
        }
    }
}
