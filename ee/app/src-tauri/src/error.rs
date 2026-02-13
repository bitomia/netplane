use std::fmt::Display;

#[derive(Debug)]
pub enum AppErrors {
    NoServer,
    AuthKey,
    AuthCode,
}

impl Display for AppErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.camel_case())
    }
}

impl AppErrors {
    fn camel_case(&self) -> String {
        let string = format!("{:?}", self);
        let mut chars = string.chars();
        chars
            .next()
            .into_iter()
            .flat_map(|c| c.to_lowercase())
            .chain(chars)
            .collect()
    }
}
