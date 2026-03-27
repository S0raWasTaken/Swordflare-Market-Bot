#[macro_export]
macro_rules! embed {
    (
        $(title: $title:expr,)?
        $(description: $description:expr,)?
        $(ephemeral: $ephemeral:expr,)?
        $(mentions: $mentions:expr,)?
        $(reply: $reply:expr,)?
    ) => {{
        poise::CreateReply {
            embeds: vec![
                poise::serenity_prelude::CreateEmbed::new()
                    $(.title($title))?
                    $(.description($description))?
            ],
            $(ephemeral: Some($ephemeral),)?
            $(reply: $reply,)?
            $(allowed_mentions: $mentions,)?
            ..Default::default()
        }

    }};
}

/// Reads environment variables and returns them as a tuple.
///
/// # Early Return
/// If any variable is not set, this macro **returns early** from the enclosing
/// function with `Err(format!("{var} must be set!").into())`.
///
/// # Requirements
/// Must be used inside a function returning `Result<T, E>` where `E: From<String>`.
#[macro_export]
macro_rules! get_vars {
    ($($var:expr),*) => {
        ($({
            let Ok(var) = std::env::var($var) else {
                return Err(format!("{} must be set!", $var).into());
            };
            var
        }),*)
    };
}

/// Logs a timestamped message to stdout or stderr.
///
/// # Usage
/// ```
/// log!("Connected from {address}");       // stdout
/// log!(e "Auth failed for {address}");    // stderr
/// ```
#[macro_export]
macro_rules! log {
    (e $($arg:tt)*) => {
        eprintln!("[{}] {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), format_args!($($arg)*))
    };
    ($($arg:tt)*) => {
        println!("[{}] {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), format_args!($($arg)*))
    };
}
