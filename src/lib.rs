//! Fast `.env` file parser with variable interpolation, multi-file layering, and type-safe loading.
//!
//! # Quick Start
//!
//! ```no_run
//! use philiprehberger_dotenv::DotEnv;
//!
//! let env = DotEnv::load().expect("failed to load .env");
//! let port: u16 = env.get_as("PORT").expect("invalid PORT");
//! let debug: bool = env.get_bool("DEBUG").expect("invalid DEBUG");
//! ```
//!
//! # Features
//!
//! - Parse `.env` files with quoted values, comments, and escape sequences
//! - Variable interpolation using `${VAR_NAME}` syntax
//! - Multi-file layering with priority ordering
//! - Type-safe accessors for common types
//! - Required variable validation

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::str::FromStr;

/// Errors that can occur when loading or accessing environment variables.
#[derive(Debug)]
pub enum DotEnvError {
    /// An I/O error occurred while reading a file.
    Io(std::io::Error),
    /// A parse error occurred at a specific line.
    Parse {
        /// The 1-based line number where the error occurred.
        line: usize,
        /// A description of the parse error.
        message: String,
    },
    /// One or more required variables are missing.
    MissingVars(Vec<String>),
    /// A value could not be converted to the requested type.
    TypeConversion {
        /// The environment variable key.
        key: String,
        /// The expected type name.
        expected: &'static str,
        /// The actual value that failed conversion.
        value: String,
    },
    /// A variable interpolation error occurred (e.g., circular reference).
    InterpolationError {
        /// The key being resolved.
        key: String,
        /// The reference that caused the error.
        references: String,
    },
}

impl std::fmt::Display for DotEnvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DotEnvError::Io(err) => write!(f, "I/O error: {err}"),
            DotEnvError::Parse { line, message } => {
                write!(f, "parse error at line {line}: {message}")
            }
            DotEnvError::MissingVars(vars) => {
                write!(f, "missing required variables: {}", vars.join(", "))
            }
            DotEnvError::TypeConversion {
                key,
                expected,
                value,
            } => {
                write!(
                    f,
                    "cannot convert {key}={value:?} to type {expected}"
                )
            }
            DotEnvError::InterpolationError { key, references } => {
                write!(
                    f,
                    "circular reference resolving {key}: references {references}"
                )
            }
        }
    }
}

impl std::error::Error for DotEnvError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DotEnvError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for DotEnvError {
    fn from(err: std::io::Error) -> Self {
        DotEnvError::Io(err)
    }
}

/// Parse raw `.env` file content into key-value pairs.
///
/// Supports:
/// - `KEY=value` (unquoted, trims trailing whitespace)
/// - `KEY="value"` (double-quoted, supports `\n`, `\t`, `\\`, `\"`)
/// - `KEY='value'` (single-quoted, literal)
/// - `KEY=` (empty value)
/// - `export KEY=value` (optional export prefix)
/// - Comments: lines starting with `#`, inline `#` after unquoted values
/// - Lines without `=` are silently skipped
fn parse_env_content(content: &str) -> Result<Vec<(String, String)>, DotEnvError> {
    let mut pairs = Vec::new();

    for (line_idx, raw_line) in content.lines().enumerate() {
        let line = raw_line.trim();

        // Skip empty lines and comment lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Strip optional `export ` prefix
        let line = if let Some(rest) = line.strip_prefix("export ") {
            rest.trim_start()
        } else {
            line
        };

        // Find the `=` separator
        let eq_pos = match line.find('=') {
            Some(pos) => pos,
            None => continue, // skip lines without =
        };

        let key = line[..eq_pos].trim().to_string();
        if key.is_empty() {
            continue;
        }

        let raw_value = &line[eq_pos + 1..];
        let value = parse_value(raw_value, line_idx + 1)?;

        pairs.push((key, value));
    }

    Ok(pairs)
}

/// Parse a value portion of a KEY=value line.
fn parse_value(raw: &str, line_number: usize) -> Result<String, DotEnvError> {
    let trimmed = raw.trim_start();

    if trimmed.is_empty() {
        return Ok(String::new());
    }

    if trimmed.starts_with('"') {
        // Double-quoted value
        parse_double_quoted(trimmed, line_number)
    } else if trimmed.starts_with('\'') {
        // Single-quoted value (literal, no escapes)
        parse_single_quoted(trimmed, line_number)
    } else {
        // Unquoted value — strip inline comments and trailing whitespace
        let value = if let Some(comment_pos) = find_inline_comment(trimmed) {
            trimmed[..comment_pos].trim_end()
        } else {
            trimmed.trim_end()
        };
        Ok(value.to_string())
    }
}

/// Find the position of an inline `#` comment in an unquoted value.
fn find_inline_comment(s: &str) -> Option<usize> {
    for (i, c) in s.char_indices() {
        if c == '#' && (i == 0 || s.as_bytes()[i - 1] == b' ') {
            return Some(i);
        }
    }
    None
}

/// Parse a double-quoted string, handling escape sequences.
fn parse_double_quoted(s: &str, line_number: usize) -> Result<String, DotEnvError> {
    let inner = &s[1..]; // skip opening quote
    let mut result = String::new();
    let mut chars = inner.chars();

    loop {
        match chars.next() {
            None => {
                return Err(DotEnvError::Parse {
                    line: line_number,
                    message: "unterminated double-quoted string".to_string(),
                });
            }
            Some('"') => {
                // End of quoted string
                return Ok(result);
            }
            Some('\\') => {
                match chars.next() {
                    Some('n') => result.push('\n'),
                    Some('t') => result.push('\t'),
                    Some('\\') => result.push('\\'),
                    Some('"') => result.push('"'),
                    Some(c) => {
                        // Unknown escape — keep literal
                        result.push('\\');
                        result.push(c);
                    }
                    None => {
                        return Err(DotEnvError::Parse {
                            line: line_number,
                            message: "unterminated escape sequence".to_string(),
                        });
                    }
                }
            }
            Some(c) => result.push(c),
        }
    }
}

/// Parse a single-quoted string (literal, no escape processing).
fn parse_single_quoted(s: &str, line_number: usize) -> Result<String, DotEnvError> {
    let inner = &s[1..]; // skip opening quote
    match inner.find('\'') {
        Some(end) => Ok(inner[..end].to_string()),
        None => Err(DotEnvError::Parse {
            line: line_number,
            message: "unterminated single-quoted string".to_string(),
        }),
    }
}

/// Resolve `${VAR_NAME}` interpolation in parsed values.
///
/// Looks up references in the parsed vars first, then falls back to `std::env::var`.
/// Detects circular references.
fn interpolate(
    pairs: Vec<(String, String)>,
) -> Result<Vec<(String, String)>, DotEnvError> {
    let raw_map: HashMap<String, String> = pairs.iter().cloned().collect();
    let keys: Vec<String> = pairs.iter().map(|(k, _)| k.clone()).collect();
    let mut resolved: HashMap<String, String> = HashMap::new();

    for key in &keys {
        if !resolved.contains_key(key) {
            resolve_key(key, &raw_map, &mut resolved, &mut HashSet::new())?;
        }
    }

    // Preserve insertion order
    let result: Vec<(String, String)> = pairs
        .into_iter()
        .map(|(k, _)| {
            let v = resolved.get(&k).cloned().unwrap_or_default();
            (k, v)
        })
        .collect();

    Ok(result)
}

/// Recursively resolve a single key's value, detecting circular references.
fn resolve_key(
    key: &str,
    raw_map: &HashMap<String, String>,
    resolved: &mut HashMap<String, String>,
    in_progress: &mut HashSet<String>,
) -> Result<String, DotEnvError> {
    if let Some(val) = resolved.get(key) {
        return Ok(val.clone());
    }

    if in_progress.contains(key) {
        return Err(DotEnvError::InterpolationError {
            key: key.to_string(),
            references: key.to_string(),
        });
    }

    let raw_value = match raw_map.get(key) {
        Some(v) => v.clone(),
        None => {
            // Fall back to process env
            return Ok(std::env::var(key).unwrap_or_default());
        }
    };

    in_progress.insert(key.to_string());

    let result = expand_references(&raw_value, key, raw_map, resolved, in_progress)?;

    in_progress.remove(key);
    resolved.insert(key.to_string(), result.clone());
    Ok(result)
}

/// Expand `${VAR}` references within a string value.
fn expand_references(
    value: &str,
    parent_key: &str,
    raw_map: &HashMap<String, String>,
    resolved: &mut HashMap<String, String>,
    in_progress: &mut HashSet<String>,
) -> Result<String, DotEnvError> {
    let mut result = String::new();
    let mut chars = value.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '$' && chars.peek() == Some(&'{') {
            chars.next(); // consume '{'
            let mut ref_name = String::new();
            let mut found_close = false;
            for c2 in chars.by_ref() {
                if c2 == '}' {
                    found_close = true;
                    break;
                }
                ref_name.push(c2);
            }
            if !found_close {
                // No closing brace, treat as literal
                result.push('$');
                result.push('{');
                result.push_str(&ref_name);
                continue;
            }
            // Resolve the referenced variable
            let resolved_val =
                resolve_key(&ref_name, raw_map, resolved, in_progress).map_err(|_| {
                    DotEnvError::InterpolationError {
                        key: parent_key.to_string(),
                        references: ref_name.clone(),
                    }
                })?;
            result.push_str(&resolved_val);
        } else {
            result.push(c);
        }
    }

    Ok(result)
}

/// A loaded set of environment variables parsed from `.env` files.
///
/// Use [`DotEnv::load`] to load from the default `.env` file, or
/// [`DotEnv::load_from`] for a specific path.
pub struct DotEnv {
    vars: HashMap<String, String>,
}

impl DotEnv {
    /// Load environment variables from a `.env` file in the current directory.
    ///
    /// # Errors
    ///
    /// Returns a [`DotEnvError`] if the file cannot be read or parsed.
    pub fn load() -> Result<DotEnv, DotEnvError> {
        DotEnv::load_from(".env")
    }

    /// Load environment variables from a specific file path.
    ///
    /// # Errors
    ///
    /// Returns a [`DotEnvError`] if the file cannot be read or parsed.
    pub fn load_from(path: impl AsRef<Path>) -> Result<DotEnv, DotEnvError> {
        let content = fs::read_to_string(path.as_ref())?;
        DotEnv::from_str(&content)
    }

    /// Load environment variables from multiple files, with later files overriding earlier ones.
    ///
    /// Files that do not exist are silently skipped.
    ///
    /// # Errors
    ///
    /// Returns a [`DotEnvError`] if any existing file cannot be parsed.
    pub fn load_layered(paths: &[impl AsRef<Path>]) -> Result<DotEnv, DotEnvError> {
        let mut all_pairs: Vec<(String, String)> = Vec::new();

        for path in paths {
            let path = path.as_ref();
            if !path.exists() {
                continue;
            }
            let content = fs::read_to_string(path)?;
            let pairs = parse_env_content(&content)?;
            all_pairs.extend(pairs);
        }

        let resolved = interpolate(all_pairs)?;
        let vars: HashMap<String, String> = resolved.into_iter().collect();
        Ok(DotEnv { vars })
    }

    /// Create a `DotEnv` from a string of `.env`-formatted content.
    fn from_str(content: &str) -> Result<DotEnv, DotEnvError> {
        let pairs = parse_env_content(content)?;
        let resolved = interpolate(pairs)?;
        let vars: HashMap<String, String> = resolved.into_iter().collect();
        Ok(DotEnv { vars })
    }

    /// Get the raw string value for a key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.vars.get(key).map(|s| s.as_str())
    }

    /// Get the value for a key, or return a default if the key is not present.
    pub fn get_or(&self, key: &str, default: &str) -> String {
        self.vars
            .get(key)
            .cloned()
            .unwrap_or_else(|| default.to_string())
    }

    /// Get a value parsed as a specific type.
    ///
    /// # Errors
    ///
    /// Returns [`DotEnvError::MissingVars`] if the key is not found, or
    /// [`DotEnvError::TypeConversion`] if the value cannot be parsed.
    pub fn get_as<T: FromStr>(&self, key: &str) -> Result<T, DotEnvError> {
        let value = self.vars.get(key).ok_or_else(|| {
            DotEnvError::MissingVars(vec![key.to_string()])
        })?;
        value.parse::<T>().map_err(|_| DotEnvError::TypeConversion {
            key: key.to_string(),
            expected: std::any::type_name::<T>(),
            value: value.clone(),
        })
    }

    /// Parse a boolean value from common representations.
    ///
    /// Accepted values (case-insensitive): `true`, `false`, `1`, `0`, `yes`, `no`.
    ///
    /// # Errors
    ///
    /// Returns [`DotEnvError::MissingVars`] if the key is not found, or
    /// [`DotEnvError::TypeConversion`] if the value is not a recognized boolean.
    pub fn get_bool(&self, key: &str) -> Result<bool, DotEnvError> {
        let value = self.vars.get(key).ok_or_else(|| {
            DotEnvError::MissingVars(vec![key.to_string()])
        })?;
        match value.to_lowercase().as_str() {
            "true" | "1" | "yes" => Ok(true),
            "false" | "0" | "no" => Ok(false),
            _ => Err(DotEnvError::TypeConversion {
                key: key.to_string(),
                expected: "bool",
                value: value.clone(),
            }),
        }
    }

    /// Get a value parsed as a specific type, returning a default if the key is missing or
    /// the value cannot be parsed.
    ///
    /// This is the fallible counterpart to [`get_as`](DotEnv::get_as): instead of returning
    /// an error, it silently falls back to `default`.
    pub fn get_or_default<T: FromStr>(&self, key: &str, default: T) -> T {
        self.vars
            .get(key)
            .and_then(|v| v.parse::<T>().ok())
            .unwrap_or(default)
    }

    /// Split a value by the given separator into a list of strings.
    ///
    /// Returns an empty vector if the key is not found.
    pub fn get_list(&self, key: &str, separator: char) -> Vec<String> {
        match self.vars.get(key) {
            Some(value) => value
                .split(separator)
                .map(|s| s.trim().to_string())
                .collect(),
            None => Vec::new(),
        }
    }

    /// Validate that all specified keys are present.
    ///
    /// # Errors
    ///
    /// Returns [`DotEnvError::MissingVars`] listing all missing keys.
    pub fn require(&self, keys: &[&str]) -> Result<(), DotEnvError> {
        let missing: Vec<String> = keys
            .iter()
            .filter(|k| !self.vars.contains_key(**k))
            .map(|k| k.to_string())
            .collect();

        if missing.is_empty() {
            Ok(())
        } else {
            Err(DotEnvError::MissingVars(missing))
        }
    }

    /// Set all loaded variables into the process environment via [`std::env::set_var`].
    pub fn apply(&self) {
        for (key, value) in &self.vars {
            // SAFETY: We are setting env vars in a controlled context.
            // In production use, callers should ensure this is called
            // before spawning threads.
            unsafe {
                std::env::set_var(key, value);
            }
        }
    }

    /// Return an iterator over all keys.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.vars.keys().map(|s| s.as_str())
    }

    /// Return an iterator over all key-value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.vars.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }
}

/// Load environment variables from a `.env` file in the current directory.
///
/// Shorthand for [`DotEnv::load`].
///
/// # Errors
///
/// Returns a [`DotEnvError`] if the file cannot be read or parsed.
pub fn load() -> Result<DotEnv, DotEnvError> {
    DotEnv::load()
}

/// Load environment variables from `.env` and set them into the process environment.
///
/// # Errors
///
/// Returns a [`DotEnvError`] if the file cannot be read or parsed.
pub fn load_and_apply() -> Result<(), DotEnvError> {
    let env = DotEnv::load()?;
    env.apply();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn parse(content: &str) -> DotEnv {
        DotEnv::from_str(content).expect("failed to parse")
    }

    #[test]
    fn test_basic_key_value() {
        let env = parse("FOO=bar\nBAZ=qux");
        assert_eq!(env.get("FOO"), Some("bar"));
        assert_eq!(env.get("BAZ"), Some("qux"));
    }

    #[test]
    fn test_double_quoted_value() {
        let env = parse(r#"GREETING="hello world""#);
        assert_eq!(env.get("GREETING"), Some("hello world"));
    }

    #[test]
    fn test_single_quoted_value() {
        let env = parse("PATH_VAR='/usr/local/bin'");
        assert_eq!(env.get("PATH_VAR"), Some("/usr/local/bin"));
    }

    #[test]
    fn test_escape_sequences_in_double_quotes() {
        let env = parse(r#"MSG="line1\nline2\ttab\\slash\"quote""#);
        assert_eq!(env.get("MSG"), Some("line1\nline2\ttab\\slash\"quote"));
    }

    #[test]
    fn test_single_quotes_no_escapes() {
        let env = parse(r"LITERAL='hello\nworld'");
        assert_eq!(env.get("LITERAL"), Some(r"hello\nworld"));
    }

    #[test]
    fn test_full_line_comment() {
        let env = parse("# this is a comment\nKEY=value");
        assert_eq!(env.get("KEY"), Some("value"));
        assert!(env.vars.len() == 1);
    }

    #[test]
    fn test_inline_comment() {
        let env = parse("KEY=value # this is a comment");
        assert_eq!(env.get("KEY"), Some("value"));
    }

    #[test]
    fn test_export_prefix() {
        let env = parse("export SECRET=hunter2");
        assert_eq!(env.get("SECRET"), Some("hunter2"));
    }

    #[test]
    fn test_empty_value() {
        let env = parse("EMPTY=\nALSO_EMPTY=");
        assert_eq!(env.get("EMPTY"), Some(""));
        assert_eq!(env.get("ALSO_EMPTY"), Some(""));
    }

    #[test]
    fn test_lines_without_equals_skipped() {
        let env = parse("VALID=yes\nINVALID_LINE\nALSO_VALID=true");
        assert_eq!(env.vars.len(), 2);
        assert_eq!(env.get("VALID"), Some("yes"));
        assert_eq!(env.get("ALSO_VALID"), Some("true"));
    }

    #[test]
    fn test_variable_interpolation_simple() {
        let env = parse("HOST=localhost\nURL=http://${HOST}/api");
        assert_eq!(env.get("URL"), Some("http://localhost/api"));
    }

    #[test]
    fn test_variable_interpolation_nested() {
        let env = parse("A=hello\nB=${A}_world\nC=${B}!");
        assert_eq!(env.get("C"), Some("hello_world!"));
    }

    #[test]
    fn test_variable_interpolation_fallback_to_env() {
        // Set a process env var and reference it
        unsafe { std::env::set_var("DOTENV_TEST_FALLBACK", "from_env"); }
        let env = parse("REF=${DOTENV_TEST_FALLBACK}");
        assert_eq!(env.get("REF"), Some("from_env"));
        unsafe { std::env::remove_var("DOTENV_TEST_FALLBACK"); }
    }

    #[test]
    fn test_dollar_without_braces_not_expanded() {
        let env = parse("VAL=hello\nREF=$VAL");
        assert_eq!(env.get("REF"), Some("$VAL"));
    }

    #[test]
    fn test_circular_reference_detection() {
        let result = DotEnv::from_str("A=${B}\nB=${A}");
        assert!(result.is_err());
        if let Err(DotEnvError::InterpolationError { .. }) = result {
            // expected
        } else {
            panic!("expected InterpolationError");
        }
    }

    #[test]
    fn test_layered_loading() {
        let dir = std::env::temp_dir().join("dotenv_test_layered");
        let _ = fs::create_dir_all(&dir);

        let base_path = dir.join("base.env");
        let override_path = dir.join("override.env");

        let mut f1 = fs::File::create(&base_path).unwrap();
        writeln!(f1, "A=base_a\nB=base_b").unwrap();

        let mut f2 = fs::File::create(&override_path).unwrap();
        writeln!(f2, "B=override_b\nC=new_c").unwrap();

        let env = DotEnv::load_layered(&[&base_path, &override_path]).unwrap();
        assert_eq!(env.get("A"), Some("base_a"));
        assert_eq!(env.get("B"), Some("override_b"));
        assert_eq!(env.get("C"), Some("new_c"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_get_as_u16() {
        let env = parse("PORT=8080");
        let port: u16 = env.get_as("PORT").unwrap();
        assert_eq!(port, 8080);
    }

    #[test]
    fn test_get_as_invalid_type() {
        let env = parse("PORT=not_a_number");
        let result: Result<u16, _> = env.get_as("PORT");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_bool_variants() {
        let env = parse("A=true\nB=false\nC=1\nD=0\nE=yes\nF=no\nG=TRUE\nH=Yes");
        assert!(env.get_bool("A").unwrap());
        assert!(!env.get_bool("B").unwrap());
        assert!(env.get_bool("C").unwrap());
        assert!(!env.get_bool("D").unwrap());
        assert!(env.get_bool("E").unwrap());
        assert!(!env.get_bool("F").unwrap());
        assert!(env.get_bool("G").unwrap());
        assert!(env.get_bool("H").unwrap());
    }

    #[test]
    fn test_get_bool_invalid() {
        let env = parse("VAL=maybe");
        assert!(env.get_bool("VAL").is_err());
    }

    #[test]
    fn test_require_all_present() {
        let env = parse("A=1\nB=2\nC=3");
        assert!(env.require(&["A", "B", "C"]).is_ok());
    }

    #[test]
    fn test_require_missing() {
        let env = parse("A=1");
        let result = env.require(&["A", "B", "C"]);
        match result {
            Err(DotEnvError::MissingVars(vars)) => {
                assert!(vars.contains(&"B".to_string()));
                assert!(vars.contains(&"C".to_string()));
                assert_eq!(vars.len(), 2);
            }
            _ => panic!("expected MissingVars error"),
        }
    }

    #[test]
    fn test_get_list() {
        let env = parse("HOSTS=a,b,c");
        let list = env.get_list("HOSTS", ',');
        assert_eq!(list, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_get_list_with_spaces() {
        let env = parse("ITEMS=one , two , three");
        let list = env.get_list("ITEMS", ',');
        assert_eq!(list, vec!["one", "two", "three"]);
    }

    #[test]
    fn test_get_list_missing_key() {
        let env = parse("OTHER=val");
        let list = env.get_list("MISSING", ',');
        assert!(list.is_empty());
    }

    #[test]
    fn test_get_or_default() {
        let env = parse("A=hello");
        assert_eq!(env.get_or("A", "default"), "hello");
        assert_eq!(env.get_or("MISSING", "fallback"), "fallback");
    }

    #[test]
    fn test_apply_sets_env_vars() {
        let env = parse("DOTENV_TEST_APPLY=applied_value");
        env.apply();
        assert_eq!(
            std::env::var("DOTENV_TEST_APPLY").unwrap(),
            "applied_value"
        );
        unsafe { std::env::remove_var("DOTENV_TEST_APPLY"); }
    }

    #[test]
    fn test_keys_and_iter() {
        let env = parse("X=1\nY=2");
        let mut keys: Vec<&str> = env.keys().collect();
        keys.sort();
        assert_eq!(keys, vec!["X", "Y"]);

        let mut pairs: Vec<(&str, &str)> = env.iter().collect();
        pairs.sort();
        assert_eq!(pairs, vec![("X", "1"), ("Y", "2")]);
    }

    #[test]
    fn test_load_from_file() {
        let dir = std::env::temp_dir().join("dotenv_test_load_from");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("test.env");

        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, "LOADED=yes").unwrap();

        let env = DotEnv::load_from(&path).unwrap();
        assert_eq!(env.get("LOADED"), Some("yes"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_whitespace_around_key() {
        let env = parse("  KEY  =value");
        assert_eq!(env.get("KEY"), Some("value"));
    }

    #[test]
    fn test_get_or_default_returns_parsed_value() {
        let env = parse("PORT=8080\nDEBUG=true\nRATIO=3.14");
        assert_eq!(env.get_or_default::<u16>("PORT", 3000), 8080);
        assert_eq!(env.get_or_default::<bool>("DEBUG", false), true);
        assert_eq!(env.get_or_default::<f64>("RATIO", 1.0), 3.14);
    }

    #[test]
    fn test_get_or_default_returns_default_on_missing() {
        let env = parse("OTHER=value");
        assert_eq!(env.get_or_default::<u16>("PORT", 3000), 3000);
        assert_eq!(env.get_or_default::<bool>("DEBUG", true), true);
        assert_eq!(env.get_or_default::<String>("NAME", "app".to_string()), "app");
    }

    #[test]
    fn test_get_or_default_returns_default_on_parse_failure() {
        let env = parse("PORT=not_a_number\nDEBUG=maybe");
        assert_eq!(env.get_or_default::<u16>("PORT", 3000), 3000);
        assert_eq!(env.get_or_default::<bool>("DEBUG", false), false);
    }

    #[test]
    fn test_interpolation_multiple_refs() {
        let env = parse("HOST=localhost\nPORT=5432\nURL=postgres://${HOST}:${PORT}/db");
        assert_eq!(env.get("URL"), Some("postgres://localhost:5432/db"));
    }
}
