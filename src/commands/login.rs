use momento::momento::auth::LoginResult;

pub async fn login() -> LoginResult {
    momento::momento::auth::login(|action| {
        match action {
            momento::momento::auth::LoginAction::OpenBrowser(open) => {
                // TODO: Need an early-out from the action sink.
                // match webbrowser::open(&open.url) {
                //     Ok(_) => {
                //         log::debug!("opened browser to {}", open.url);
                //     },
                //     Err(e) => {
                //         return LoginResult::NotLoggedIn(NotLoggedIn { error_message: e.to_string })
                //     },
                // }
                webbrowser::open(&open.url).expect("Unable to open browser")
            },
            momento::momento::auth::LoginAction::ShowMessage(message) => {
                println!("{}", message.text);
            },
        }
    }).await
}
