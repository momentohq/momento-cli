use momento::momento::auth::{EarlyOutActionResult, LoginAction};
use momento::{momento::auth::LoginResult, response::error::MomentoError};
use qrcode::render::unicode;
use qrcode::QrCode;
use utils::console::console_info;

#[derive(clap::ArgEnum, Clone, Debug)]
pub enum LoginMode {
    Browser,
    Qr,
}

pub async fn login(login_mode: LoginMode) -> LoginResult {
    momento::momento::auth::login(match login_mode {
        LoginMode::Browser => login_with_browser,
        LoginMode::Qr => login_with_qr_code,
    })
    .await
}

fn login_with_browser(action: LoginAction) -> EarlyOutActionResult {
    match action {
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
            console_info!("{}", message.text);
            None
        }
    }
}

fn login_with_qr_code(action: LoginAction) -> EarlyOutActionResult {
    match action {
        momento::momento::auth::LoginAction::OpenBrowser(open) => {
            console_info!("Navigate here to log in: {}", open.url);
            let code = QrCode::new(open.url).unwrap();
            let image = code
                .render::<unicode::Dense1x2>()
                .dark_color(unicode::Dense1x2::Dark)
                .light_color(unicode::Dense1x2::Light)
                .build();
            console_info!("{}", image);
            None
        }
        momento::momento::auth::LoginAction::ShowMessage(message) => {
            console_info!("{}", message.text);
            None
        }
    }
}
