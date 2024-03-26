#![allow(clippy::pub_with_shorthand)]

macro_rules! return_or_exit {
    ($result: expr, $exit: expr) => {
        match $result {
            Ok(val) => val,
            Err(err) => {
                log::error!("{err}");
                std::process::exit($exit);
            }
        }
    };
}

pub(crate) use return_or_exit;
