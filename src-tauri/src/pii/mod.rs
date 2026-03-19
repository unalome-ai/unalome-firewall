use crate::models::{Action, ActionType};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiiFinding {
    pub id: String,
    pub action_id: Option<String>,
    pub agent_id: String,
    pub finding_type: String,
    pub severity: String,
    pub description: String,
    pub source_file: Option<String>,
    pub source_context: String,
    pub redacted_value: String,
    pub recommended_action: String,
    pub timestamp: String,
    pub dismissed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiiStats {
    pub total: i64,
    pub by_severity: HashMap<String, i64>,
    pub by_type: HashMap<String, i64>,
    pub by_agent: HashMap<String, i64>,
    pub today: i64,
    pub this_week: i64,
}

struct PiiPattern {
    name: &'static str,
    finding_type: &'static str,
    severity: &'static str,
    regex: Regex,
    validator: Option<fn(&str) -> bool>,
}

pub struct PiiScanner {
    patterns: Vec<PiiPattern>,
    generic_secret_regex: Regex,
    password_regex: Regex,
    env_variable_regex: Regex,
}

impl PiiScanner {
    pub fn new() -> Self {
        let patterns = vec![
            // Anthropic key must come before OpenAI (both start with sk-)
            PiiPattern {
                name: "Anthropic API Key",
                finding_type: "api_key",
                severity: "critical",
                regex: Regex::new(r"\bsk-ant-[a-zA-Z0-9\-_]{40,}\b").unwrap(),
                validator: None,
            },
            PiiPattern {
                name: "OpenAI API Key",
                finding_type: "api_key",
                severity: "critical",
                // OpenAI keys: sk-proj-... or sk-<org>-... with 40+ chars total after sk-
                regex: Regex::new(r"\bsk-(?:proj-)?[a-zA-Z0-9]{20,}\b").unwrap(),
                validator: Some(validate_openai_key),
            },
            PiiPattern {
                name: "AWS Access Key",
                finding_type: "api_key",
                severity: "critical",
                regex: Regex::new(r"\bAKIA[0-9A-Z]{16}\b").unwrap(),
                validator: None,
            },
            PiiPattern {
                name: "GitHub Token",
                finding_type: "api_key",
                severity: "critical",
                regex: Regex::new(r"\bgh[pousr]_[a-zA-Z0-9]{36}\b").unwrap(),
                validator: None,
            },
            PiiPattern {
                name: "Stripe Key",
                finding_type: "api_key",
                severity: "critical",
                regex: Regex::new(r"\b[sp]k_live_[a-zA-Z0-9]{24,}\b").unwrap(),
                validator: None,
            },
            PiiPattern {
                name: "Google API Key",
                finding_type: "api_key",
                severity: "critical",
                regex: Regex::new(r"\bAIza[0-9A-Za-z_-]{35}\b").unwrap(),
                validator: None,
            },
            PiiPattern {
                name: "Private Key",
                finding_type: "private_key",
                severity: "critical",
                // Only match actual PEM headers, not docs/code discussing them
                regex: Regex::new(r"-----BEGIN (RSA |EC |DSA |OPENSSH |ED25519 )?PRIVATE KEY-----").unwrap(),
                validator: Some(validate_private_key),
            },
            PiiPattern {
                name: "JWT Token",
                finding_type: "jwt",
                severity: "critical",
                // Require minimum length segments to avoid short false matches
                regex: Regex::new(r"\beyJ[a-zA-Z0-9_-]{10,}\.eyJ[a-zA-Z0-9_-]{10,}\.[a-zA-Z0-9_-]{10,}\b").unwrap(),
                validator: None,
            },
            PiiPattern {
                name: "Connection String",
                finding_type: "connection_string",
                severity: "critical",
                // Require @ (user:pass@host) or at least host:port to avoid matching schema-only refs
                regex: Regex::new(r#"(mongodb\+srv|mongodb|postgresql|postgres|mysql|redis|amqp)://[^\s"']+@[^\s"']+"#).unwrap(),
                validator: None,
            },
            PiiPattern {
                name: "SSN",
                finding_type: "ssn",
                severity: "high",
                regex: Regex::new(r"\b(\d{3})-(\d{2})-(\d{4})\b").unwrap(),
                validator: Some(validate_ssn),
            },
            PiiPattern {
                name: "Credit Card",
                finding_type: "credit_card",
                severity: "high",
                // Require at least one separator to avoid matching bare 16-digit numbers (timestamps, IDs)
                regex: Regex::new(r"\b(\d{4})[-\s](\d{4})[-\s](\d{4})[-\s](\d{4})\b").unwrap(),
                validator: Some(validate_credit_card),
            },
            PiiPattern {
                name: "Email Address",
                finding_type: "email",
                severity: "medium",
                regex: Regex::new(r"\b[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}\b").unwrap(),
                validator: Some(validate_email),
            },
            PiiPattern {
                name: "Phone Number",
                finding_type: "phone",
                severity: "medium",
                regex: Regex::new(r"(?:\+?1[-.\s])?\(\d{3}\)[-.\s]?\d{3}[-.\s]\d{4}\b|\b\d{3}[-.\s]\d{3}[-.\s]\d{4}\b").unwrap(),
                validator: None,
            },
            PiiPattern {
                name: "IPv4 Address",
                finding_type: "ip_address",
                severity: "low",
                regex: Regex::new(r"\b(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})\b").unwrap(),
                validator: Some(validate_ip),
            },
        ];

        Self {
            patterns,
            // Require the value to be quoted or after = to indicate an actual assignment, not code discussion
            generic_secret_regex: Regex::new(
                r#"(?i)(secret|api_key|apikey|secret_key|access_token|auth_token)["']*\s*[:=]\s*["']([a-zA-Z0-9+/=_\-]{24,})["']"#
            ).unwrap(),
            // Only match password assignments with quoted values (actual credentials, not code patterns)
            password_regex: Regex::new(
                r#"(?i)(password|passwd|pwd)\s*[:=]\s*["']([^\s"']{8,})["']"#
            ).unwrap(),
            env_variable_regex: Regex::new(
                r"^[A-Z][A-Z0-9_]+=.+"
            ).unwrap(),
        }
    }

    pub fn scan_text(
        &self,
        text: &str,
        source_file: Option<&str>,
        agent_id: &str,
        action_id: Option<&str>,
    ) -> Vec<PiiFinding> {
        let mut findings = Vec::new();

        for line in text.lines() {
            // Check all compiled patterns
            for pattern in &self.patterns {
                for mat in pattern.regex.find_iter(line) {
                    let matched = mat.as_str();

                    // Run validator if present
                    if let Some(validator) = pattern.validator {
                        if !validator(matched) {
                            continue;
                        }
                    }

                    findings.push(PiiFinding {
                        id: Uuid::new_v4().to_string(),
                        action_id: action_id.map(|s| s.to_string()),
                        agent_id: agent_id.to_string(),
                        finding_type: pattern.finding_type.to_string(),
                        severity: pattern.severity.to_string(),
                        description: generate_description(
                            pattern.name,
                            pattern.finding_type,
                            agent_id,
                            source_file,
                        ),
                        source_file: source_file.map(|s| s.to_string()),
                        source_context: redact_line(line, matched),
                        redacted_value: redact_value(matched),
                        recommended_action: generate_recommendation(pattern.finding_type),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                        dismissed: false,
                    });
                }
            }

            // Generic secret detection (high-entropy strings near keywords)
            for caps in self.generic_secret_regex.captures_iter(line) {
                if let Some(value) = caps.get(2) {
                    let val = value.as_str();
                    if val.len() > 20 && shannon_entropy(val) > 4.5 {
                        // Skip if already caught by a specific pattern
                        let already_found = findings.iter().any(|f| {
                            line.contains(&f.redacted_value.replace("****", ""))
                                || f.source_context == redact_line(line, val)
                        });
                        if already_found {
                            continue;
                        }

                        findings.push(PiiFinding {
                            id: Uuid::new_v4().to_string(),
                            action_id: action_id.map(|s| s.to_string()),
                            agent_id: agent_id.to_string(),
                            finding_type: "api_key".to_string(),
                            severity: "critical".to_string(),
                            description: generate_description(
                                "Generic Secret/Key",
                                "api_key",
                                agent_id,
                                source_file,
                            ),
                            source_file: source_file.map(|s| s.to_string()),
                            source_context: redact_line(line, val),
                            redacted_value: redact_value(val),
                            recommended_action: generate_recommendation("api_key"),
                            timestamp: chrono::Utc::now().to_rfc3339(),
                            dismissed: false,
                        });
                    }
                }
            }

            // Password detection
            for caps in self.password_regex.captures_iter(line) {
                if let Some(value) = caps.get(2) {
                    let val = value.as_str();
                    findings.push(PiiFinding {
                        id: Uuid::new_v4().to_string(),
                        action_id: action_id.map(|s| s.to_string()),
                        agent_id: agent_id.to_string(),
                        finding_type: "password".to_string(),
                        severity: "high".to_string(),
                        description: generate_description(
                            "Password",
                            "password",
                            agent_id,
                            source_file,
                        ),
                        source_file: source_file.map(|s| s.to_string()),
                        source_context: redact_line(line, val),
                        redacted_value: redact_value(val),
                        recommended_action: generate_recommendation("password"),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                        dismissed: false,
                    });
                }
            }

            // Env variable detection (for .env file context)
            if let Some(src) = source_file {
                if src.contains(".env") && self.env_variable_regex.is_match(line) {
                    if let Some(eq_pos) = line.find('=') {
                        let val = &line[eq_pos + 1..];
                        if !val.is_empty() {
                            findings.push(PiiFinding {
                                id: Uuid::new_v4().to_string(),
                                action_id: action_id.map(|s| s.to_string()),
                                agent_id: agent_id.to_string(),
                                finding_type: "env_variable".to_string(),
                                severity: "high".to_string(),
                                description: generate_description(
                                    "Environment Variable",
                                    "env_variable",
                                    agent_id,
                                    source_file,
                                ),
                                source_file: Some(src.to_string()),
                                source_context: redact_line(line, val),
                                redacted_value: redact_value(val),
                                recommended_action: generate_recommendation("env_variable"),
                                timestamp: chrono::Utc::now().to_rfc3339(),
                                dismissed: false,
                            });
                        }
                    }
                }
            }
        }

        findings
    }

    pub fn scan_file(
        &self,
        path: &str,
        agent_id: &str,
        action_id: Option<&str>,
    ) -> Vec<PiiFinding> {
        // Skip binary files and files > 1MB
        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(_) => return Vec::new(),
        };

        if metadata.len() > 1_048_576 {
            return Vec::new();
        }

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Vec::new(), // Binary or unreadable
        };

        self.scan_text(&content, Some(path), agent_id, action_id)
    }

    pub fn scan_action(&self, action: &Action) -> Vec<PiiFinding> {
        let mut findings = Vec::new();
        let agent_id = &action.agent_id;
        let action_id = Some(action.id.as_str());

        // Scan the action description
        findings.extend(self.scan_text(
            &action.description,
            None,
            agent_id,
            action_id,
        ));

        // Scan metadata as JSON string
        let meta_str = action.metadata.to_string();
        findings.extend(self.scan_text(&meta_str, None, agent_id, action_id));

        // For file access/write actions, scan the referenced file
        match &action.action_type {
            ActionType::FileAccess { path, operation } => {
                if operation == "write" || operation == "read" {
                    findings.extend(self.scan_file(path, agent_id, action_id));
                }
            }
            ActionType::ToolCall { args, .. } => {
                // Scan tool call arguments
                let args_str = args.to_string();
                findings.extend(self.scan_text(&args_str, None, agent_id, action_id));
            }
            ActionType::Message { content } => {
                findings.extend(self.scan_text(content, None, agent_id, action_id));
            }
            _ => {}
        }

        findings
    }
}

fn redact_value(raw: &str) -> String {
    let chars: Vec<char> = raw.chars().collect();
    if chars.len() <= 8 {
        return "****".to_string();
    }
    let prefix: String = chars[..4].iter().collect();
    let suffix: String = chars[chars.len() - 4..].iter().collect();
    format!("{}****{}", prefix, suffix)
}

fn redact_line(line: &str, sensitive: &str) -> String {
    line.replace(sensitive, &redact_value(sensitive))
}

fn shannon_entropy(s: &str) -> f64 {
    let mut freq: HashMap<char, f64> = HashMap::new();
    let len = s.len() as f64;

    for c in s.chars() {
        *freq.entry(c).or_insert(0.0) += 1.0;
    }

    freq.values()
        .map(|count| {
            let p = count / len;
            -p * p.log2()
        })
        .sum()
}

fn luhn_check(digits: &str) -> bool {
    let digits: Vec<u32> = digits
        .chars()
        .filter(|c| c.is_ascii_digit())
        .filter_map(|c| c.to_digit(10))
        .collect();

    if digits.len() < 13 {
        return false;
    }

    let mut sum = 0u32;
    let mut double = false;

    for &d in digits.iter().rev() {
        let mut val = d;
        if double {
            val *= 2;
            if val > 9 {
                val -= 9;
            }
        }
        sum += val;
        double = !double;
    }

    sum % 10 == 0
}

fn validate_openai_key(s: &str) -> bool {
    // Must start with sk- and be long enough to be a real key (40+ chars)
    // Skip if it looks like a variable name or placeholder
    if s.len() < 40 {
        return false;
    }
    let after_prefix = &s[3..]; // after "sk-"
    // Real keys have mixed case + digits, not just lowercase words
    let has_digit = after_prefix.chars().any(|c| c.is_ascii_digit());
    let has_upper = after_prefix.chars().any(|c| c.is_ascii_uppercase());
    has_digit && has_upper
}

fn validate_private_key(s: &str) -> bool {
    // Only flag if this looks like an actual key header, not a reference in code/docs
    // Real PEM headers are on their own line or very close to the start
    // The regex already constrains to specific key types (RSA, EC, DSA, OPENSSH, ED25519, or bare)
    // We just verify this isn't inside a string literal that's discussing the format
    let lower = s.to_lowercase();
    // Skip common false positives: code examples, docs, error messages
    !lower.contains("example") && !lower.contains("placeholder")
}

fn validate_email(s: &str) -> bool {
    // Skip common false positives: placeholder emails, noreply, test emails, file paths
    let lower = s.to_lowercase();
    let skip_domains = [
        "example.com", "example.org", "test.com", "localhost",
        "noreply", "no-reply", "placeholder",
    ];
    for domain in &skip_domains {
        if lower.contains(domain) {
            return false;
        }
    }
    // Skip if it looks like a package scope (@types/, @babel/, etc.)
    if s.starts_with('@') && !s.contains('.') {
        return false;
    }
    true
}

fn validate_ssn(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return false;
    }
    let first = match parts[0].parse::<u32>() {
        Ok(v) => v,
        Err(_) => return false,
    };
    let second = match parts[1].parse::<u32>() {
        Ok(v) => v,
        Err(_) => return false,
    };
    let third = match parts[2].parse::<u32>() {
        Ok(v) => v,
        Err(_) => return false,
    };

    // Invalid SSN area numbers
    if first == 0 || first == 666 || (900..=999).contains(&first) {
        return false;
    }
    // Group number can't be 00
    if second == 0 {
        return false;
    }
    // Serial number can't be 0000
    if third == 0 {
        return false;
    }
    // Skip patterns that look like dates (month-day-year): 01-01-2024
    if first <= 12 && second <= 31 && (third >= 1900 && third <= 2100) {
        return false;
    }

    true
}

fn validate_credit_card(s: &str) -> bool {
    luhn_check(s)
}

fn validate_ip(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 4 {
        return false;
    }

    // Parse octets
    let octets: Vec<u8> = match parts.iter().map(|p| p.parse::<u8>()).collect() {
        Ok(o) => o,
        Err(_) => return false,
    };

    // Skip private/loopback/link-local/reserved ranges
    if octets[0] == 127                                          // loopback
        || (octets[0] == 0 && octets[1] == 0 && octets[2] == 0 && octets[3] == 0) // unspecified
        || octets[0] == 10                                       // private class A
        || (octets[0] == 192 && octets[1] == 168)               // private class C
        || (octets[0] == 172 && (16..=31).contains(&octets[1])) // private class B
        || (octets[0] == 169 && octets[1] == 254)               // link-local
        || octets[0] == 255                                      // broadcast
        || octets[0] == 0                                        // "this" network
    {
        return false;
    }

    // Skip version-number-like patterns: all octets <= 20 (e.g., 1.2.3.4, 2.0.0.1)
    if octets.iter().all(|&o| o <= 20) {
        return false;
    }

    true
}

fn generate_description(
    pattern_name: &str,
    finding_type: &str,
    agent_id: &str,
    source_file: Option<&str>,
) -> String {
    let location = source_file
        .map(|f| format!(" in {}", f))
        .unwrap_or_default();

    match finding_type {
        "api_key" => format!(
            "{} detected{} during {} activity. This key could be used for unauthorized access if exposed.",
            pattern_name, location, agent_id
        ),
        "private_key" => format!(
            "Private key found{} during {} activity. Private keys must never be shared or committed to version control.",
            location, agent_id
        ),
        "jwt" => format!(
            "JWT token found{} during {} activity. JWTs may contain session data and should not be logged.",
            location, agent_id
        ),
        "connection_string" => format!(
            "Database connection string found{} during {} activity. Connection strings contain credentials.",
            location, agent_id
        ),
        "ssn" => format!(
            "Social Security Number detected{} during {} activity. SSNs are highly sensitive personal data.",
            location, agent_id
        ),
        "credit_card" => format!(
            "Credit card number detected{} during {} activity. Card numbers are regulated under PCI-DSS.",
            location, agent_id
        ),
        "password" => format!(
            "Password value detected{} during {} activity. Passwords should never appear in plaintext.",
            location, agent_id
        ),
        "env_variable" => format!(
            "Environment variable with sensitive value found{} during {} activity.",
            location, agent_id
        ),
        "email" => format!(
            "Email address found{} during {} activity. Emails are personal identifiable information.",
            location, agent_id
        ),
        "phone" => format!(
            "Phone number found{} during {} activity. Phone numbers are personal identifiable information.",
            location, agent_id
        ),
        "ip_address" => format!(
            "Public IP address found{} during {} activity.",
            location, agent_id
        ),
        _ => format!(
            "Sensitive data detected{} during {} activity.",
            location, agent_id
        ),
    }
}

fn generate_recommendation(finding_type: &str) -> String {
    match finding_type {
        "api_key" => "Rotate this API key immediately. Use environment variables or a secrets manager instead of hardcoding keys.".to_string(),
        "private_key" => "Remove the private key from this location. Store it in a secure key vault and never commit it to version control.".to_string(),
        "jwt" => "Avoid logging JWT tokens. If this token was exposed, invalidate the session and issue a new one.".to_string(),
        "connection_string" => "Move database credentials to environment variables or a secrets manager. Avoid hardcoded connection strings.".to_string(),
        "ssn" => "Remove SSN data immediately. If processing SSNs is required, ensure proper encryption and access controls are in place.".to_string(),
        "credit_card" => "Remove credit card numbers. Use a PCI-compliant payment processor like Stripe instead of handling card data directly.".to_string(),
        "password" => "Never store passwords in plaintext. Use a proper hashing algorithm (bcrypt, argon2) and store only the hash.".to_string(),
        "env_variable" => "Ensure .env files are in .gitignore. Never commit environment files with sensitive values to version control.".to_string(),
        "email" => "Consider whether this email needs to be stored or logged. Minimize PII retention where possible.".to_string(),
        "phone" => "Consider whether this phone number needs to be stored or logged. Minimize PII retention where possible.".to_string(),
        "ip_address" => "Consider whether logging this IP address is necessary. IP addresses can be considered PII under GDPR.".to_string(),
        _ => "Review this finding and take appropriate action to protect sensitive data.".to_string(),
    }
}
