#[macro_export]
macro_rules! git_rev_main {
    () => {{
        let str = include_str!("../../../.git/refs/heads/main");
        str.chars().take(8).collect::<String>()
    }};
}
