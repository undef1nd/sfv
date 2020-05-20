pub fn is_tchar(c: &char) -> bool {
    // See tchar values list in https://tools.ietf.org/html/rfc7230#section-3.2.6
    let tchars = "!#$%&'*+-.^_'|~";
    tchars.contains(*c) || c.is_ascii_alphanumeric()
}
