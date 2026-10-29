// horizon-essentials: a port of EssentialsX (GPL-3.0) to Horizon. Pure logic, no Horizon types:
// this file is the injected module `crate::game::ess_core` and, unchanged, part of crates/ess-standalone.

/// `/ping [text]`: "Pong!", or the text echoed back (Commandping).
pub fn ping(args: &str) -> String {
    let text = args.trim();
    if text.is_empty() { "Pong!".to_string() } else { text.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_answers_pong() {
        assert_eq!(ping(""), "Pong!");
        assert_eq!(ping("  hello there "), "hello there");
    }
}
