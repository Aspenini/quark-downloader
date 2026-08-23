//! Tiny URL splitter for playlist detection. Not a general URL library.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UrlParts<'a> {
    pub host: &'a str,
    pub path: &'a str,
    pub query: &'a str,
}

pub fn split(url: &str) -> Option<UrlParts<'_>> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let (host_and_maybe_user, path_query) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let host = host_and_maybe_user
        .rsplit('@')
        .next()
        .unwrap_or(host_and_maybe_user);
    let host = host.split(':').next().unwrap_or(host);
    if host.is_empty() {
        return None;
    }
    let (path, query) = match path_query.split_once('?') {
        Some((p, q)) => (p, q.split('#').next().unwrap_or(q)),
        None => (path_query.split('#').next().unwrap_or(path_query), ""),
    };
    Some(UrlParts { host, path, query })
}

pub fn query_has(query: &str, key: &str) -> bool {
    query.split('&').any(|pair| {
        let name = pair.split('=').next().unwrap_or("");
        name.eq_ignore_ascii_case(key)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_host_path_query() {
        let u = split("https://www.youtube.com/watch?v=abc&list=PLx").unwrap();
        assert_eq!(u.host, "www.youtube.com");
        assert_eq!(u.path, "/watch");
        assert!(query_has(u.query, "v"));
        assert!(query_has(u.query, "list"));
    }

    #[test]
    fn rejects_non_urls() {
        assert!(split("not a url").is_none());
    }
}
