use kend_gmail::{Account, CancelToken};
use rune::{Align, Cx, El, col, el, text};
use rune_kit::{Button, ButtonVariant, Kit, Spinner};

use crate::app::{App, Auth};

type Scene = (&'static str, fn(&mut App));

/// Named states for headless snapshots (`--snap out.png --scene signed-in`).
pub const SCENES: &[Scene] = &[
    ("not-configured", |s| s.auth = Auth::NotConfigured),
    ("signed-out", |s| s.auth = Auth::SignedOut { error: None }),
    ("sign-in-failed", |s| {
        s.auth = Auth::SignedOut {
            error: Some("sign-in timed out".into()),
        }
    }),
    ("signing-in", |s| {
        s.auth = Auth::SigningIn {
            attempt: 1,
            cancel: CancelToken::default(),
        }
    }),
    ("signed-in", |s| {
        s.auth = Auth::SignedIn {
            account: Account::new("you@gmail.com".into(), String::new()),
        }
    }),
];

pub fn view(s: &App, cx: &mut Cx<App>) -> El {
    let k = Kit::neutral(cx.system_dark());
    let content = match &s.auth {
        Auth::Loading => el(),
        Auth::NotConfigured => panel(
            &k,
            "Gmail isn't set up",
            "Add your Google client ID and secret to .env (see .env.example), then rebuild.",
        ),
        Auth::SignedOut { error } => {
            let mut p = panel(
                &k,
                "Welcome to kend",
                "Sign in with your Google account to load your mail.",
            )
            .child(
                Button::new("Sign in with Google")
                    .variant(ButtonVariant::Contrast)
                    .on_press(|s: &mut App| s.start_sign_in())
                    .build(&k)
                    .key("sign-in"),
            );
            if let Some(error) = error {
                p = p.child(message(&k, error).color(k.c.danger.fg).key("error"));
            }
            p
        }
        Auth::SigningIn { .. } => panel(
            &k,
            "Finish signing in in your browser",
            "kend will continue once you allow access.",
        )
        .child(Spinner::new().build(&k))
        .child(
            Button::new("Cancel")
                .variant(ButtonVariant::Ghost)
                .on_press(|s: &mut App| s.cancel_sign_in())
                .build(&k)
                .key("cancel"),
        ),
        Auth::SignedIn { account } => panel(
            &k,
            "Inbox",
            &format!("Signed in as {}. No messages yet.", account.email),
        )
        .child(
            Button::new("Sign out")
                .variant(ButtonVariant::Ghost)
                .on_press(|s: &mut App| s.sign_out())
                .build(&k)
                .key("sign-out"),
        ),
    };
    col()
        .w_full()
        .h_full()
        .center()
        .fill(k.c.bg_base)
        .child(content)
}

fn panel(k: &Kit, title: &str, body: &str) -> El {
    let title = text(title)
        .key("title")
        .font(k.ty.ui_medium)
        .color(k.c.text_base);
    col()
        .w(360.0)
        .center()
        .gap(12.0)
        .child(title)
        .child(message(k, body).key("body"))
}

fn message(k: &Kit, s: &str) -> El {
    text(s)
        .font(k.ty.ui)
        .color(k.c.text_muted)
        .wrap()
        .align(Align::Center)
        .w_full()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune::{Env, Runtime};

    fn render(scene: &str) -> Runtime<App> {
        let mut app = App::with_client(None);
        let (_, set) = SCENES.iter().find(|(name, _)| *name == scene).unwrap();
        set(&mut app);
        let mut rt = Runtime::new(
            app,
            view,
            (1100.0, 720.0),
            Env {
                system_dark: true,
                scale: 2.0,
            },
        );
        rt.frame(0.0);
        rt
    }

    #[test]
    fn each_sign_in_state_shows_its_controls() {
        assert!(render("not-configured").find("sign-in").is_none());
        assert!(render("signed-out").find("sign-in").is_some());
        assert!(render("sign-in-failed").find("error").is_some());
        assert!(render("signing-in").find("cancel").is_some());
        assert!(render("signed-in").find("sign-out").is_some());
    }

    #[test]
    fn cancel_returns_to_the_sign_in_button() {
        let mut rt = render("signing-in");
        let (x, y) = rt.find("cancel").unwrap();
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(1.0 / 60.0);
        assert!(matches!(rt.state.auth, Auth::SignedOut { error: None }));
    }
}
