#[inline]
const fn invalid_label_key_start_character(c: char) -> bool {
    // Essentially, needs to match the regex pattern of [a-zA-Z_].
    !(c.is_ascii_alphabetic() || c == '_')
}

#[inline]
const fn invalid_label_key_character(c: char) -> bool {
    // Essentially, needs to match the regex pattern of [a-zA-Z0-9_].
    !(c.is_ascii_alphanumeric() || c == '_')
}

pub fn sanitize_label_key(key: &str) -> String {
    // The first character must be [a-zA-Z_], and all subsequent characters must be [a-zA-Z0-9_].
    key.replacen(invalid_label_key_start_character, "_", 1)
        .replace(invalid_label_key_character, "_")
        .replacen("__", "___", 1)
}

pub fn sanitize_label_value(value: &str) -> String {
    sanitize_label_value_or_descpiption(value, false)
}

fn sanitize_label_value_or_descpiption(value: &str, is_desc: bool) -> String {
    // All Unicode characters are valid, but backslashes, double quotes, and line feeds must be
    // escaped.
    let mut sanitized = String::with_capacity(value.len());

    let mut previous_backslash = false;
    for c in value.chars() {
        match c {
            // Any raw newlines get escaped, period.
            '\n' => sanitized.push_str("\\n"),
            // Any double quote we see gets escaped, but only for label values, not descriptions.
            '"' if !is_desc => {
                previous_backslash = false;
                sanitized.push_str("\\\"");
            }
            // If we see a backslash, we might be either seeing one that is being used to escape
            // something, or seeing one that has being escaped. If our last character was a
            // backslash, then we know this one has already been escaped, and we just emit the
            // escaped backslash.
            '\\' => {
                if previous_backslash {
                    // This backslash was preceded by another backslash, so we can safely emit an
                    // escaped backslash.
                    sanitized.push_str("\\\\");
                }

                // This may or may not be a backslash that is about to escape something else, so if
                // we toggle the value here: if it was false, then we're marking ourselves as having
                // seen a previous backslash (duh) or we just emitted an escaped backslash and now
                // we're clearing the flag.
                previous_backslash = !previous_backslash;
            }
            c => {
                // If we had a backslash in holding, and we're here, we know it wasn't escaping
                // something we care about, so it's on its own, and we emit an escaped backslash,
                // before emitting the actual character we're handling.
                if previous_backslash {
                    previous_backslash = false;
                    sanitized.push_str("\\\\");
                }
                sanitized.push(c);
            }
        }
    }

    // Handle any dangling backslash by writing it out in an escaped fashion.
    if previous_backslash {
        sanitized.push_str("\\\\");
    }

    sanitized
}
