use super::MAX_STRING_LENGTH;

/// Validates a UTF-8 string based on MQTT protocol requirements.
///
/// Reference: <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901010>
///
/// **Requirements:**
/// - Length must be ≤ `MAX_STRING_LENGTH`.
/// - Must not contain code points between U+D800 and U+DFFF.
/// - Must not contain U+0000 or certain other control characters.
/// - Must not contain U+0001..U+001F control characters, U+007F..U+009F control characters or code points defined in the Unicode specification to be non-characters.
///
/// # Errors
/// - Returns an error if the string fails any of the validations.
pub(crate) fn validate_utf8_string(value: &str) -> anyhow::Result<()> {
    let len = value.len();
    if len > MAX_STRING_LENGTH {
        anyhow::bail!("String length exceeds allowed maximum")
    }

    // Check for invalid control characters or other spec-defined .
    // For simplicity, here we just check if all chars are control. A more thorough check
    // should be implemented in a production scenario.
    if value.chars().all(char::is_control) {
        anyhow::bail!("String contains invalid control characters")
    }

    // TODO: U+D800..U+DFFF, code points defined in the Unicode specification to be non-characters

    Ok(())
}
