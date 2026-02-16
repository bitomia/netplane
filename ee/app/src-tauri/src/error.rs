use std::fmt::Display;

#[derive(Debug)]
pub enum AppError {
    NoServer,
    AuthKey,
    AuthCode,
    GenericError,
}

impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = format!("{:?}", self);
        let mut chars = string.chars();
        let show: String = chars
            .next()
            .into_iter()
            .flat_map(|c| c.to_lowercase())
            .chain(chars)
            .collect();

        write!(f, "{}", show)
    }
}
