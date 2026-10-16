pub fn search_argument(input: &str) -> Result<String, String> {
    let input = input.trim();
    if input.is_empty() || input.len() > 2048 || input.chars().any(char::is_control) {
        return Err("Masukkan pencarian yang valid".into());
    }
    if input.contains("://") {
        let rest = input.strip_prefix("https://").ok_or("URL harus HTTPS")?;
        let host = rest.split('/').next().unwrap_or("");
        if !matches!(
            host,
            "youtube.com" | "www.youtube.com" | "music.youtube.com" | "youtu.be"
        ) {
            return Err("Hanya URL YouTube yang didukung".into());
        }
        Ok(input.to_owned())
    } else {
        Ok(format!("ytsearch20:{input}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn media_input_rejects_foreign_hosts_and_keeps_search_as_data() {
        assert!(search_argument("https://evil.example/watch?v=123").is_err());
        assert_eq!(
            search_argument("-live music").unwrap(),
            "ytsearch20:-live music"
        );
        assert_eq!(
            search_argument("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap(),
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
        );
        assert!(search_argument(" ").is_err());
    }
}
