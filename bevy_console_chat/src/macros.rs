/// Reply with the [`format!`] syntax.
///
/// # Example
///
/// ```ignore
/// reply!(cmd, "Hello, {}", name);
/// ```
#[macro_export]
macro_rules! reply {
    ($cmd: ident, $fmt: literal$(, $($arg:expr),* $(,)?)?) => {
        {
            let msg = format!($fmt$(, $($arg),*)?);
            $cmd.reply(msg);
        }
    };
}

/// Reply with the [`format!`] syntax followed by `[ok]`.
///
/// # Example
///
/// ```ignore
/// reply_ok!(cmd, "Hello, {}", name);
/// ```
#[macro_export]
macro_rules! reply_ok {
    ($cmd: ident, $fmt: literal$(, $($arg:expr),* $(,)?)?) => {
        {
            let msg = format!($fmt$(, $($arg),*)?);
            $cmd.reply_ok(msg);
        }
    };
}

/// Reply with the [`format!`] syntax followed by `[failed]`.
///
/// # Example
///
/// ```ignore
/// reply_failed!(cmd, "Hello, {}", name);
/// ```
#[macro_export]
macro_rules! reply_failed {
    ($cmd: ident, $fmt: literal$(, $($arg:expr),* $(,)?)?) => {
        {
            let msg = format!($fmt$(, $($arg),*)?);
            $cmd.reply_failed(msg);
        }
    };
}
