//! Application specific messages.
//! 
//! Used code from this repo: https://github.com/behai-nguyen/rust_web_01.git // TODO: not used yet (only examples code)

pub static LOGIN_FAILURE_MSG: &str = "Please check login detail.";
pub static UNAUTHORISED_ACCESS_MSG: &str = "Please log in first.";
pub static TOKEN_INVALID_MSG: &str = "Invalid token.";
pub static TOKEN_EXPIRED_MSG: &str = "Token has expired.";
pub static TOKEN_OTHER_ERR_MSG: &str = "Token is in error.";
pub static TOKEN_STR_JWT_MSG: &str = "JWT extension should be a string.";