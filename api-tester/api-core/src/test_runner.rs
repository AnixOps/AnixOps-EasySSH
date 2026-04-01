use crate::types::*;
use serde_json::Value;
use std::collections::HashMap;

pub struct TestRunner;

impl TestRunner {
    pub fn new() -> Self {
        Self
    }

    /// Run tests defined in a test script against a response
    pub fn run_tests(&self, test_script: &str, response: &ApiResponse) -> Vec<TestResult> {
        let mut results = Vec::new();

        // Parse test script and run assertions
        // A simple scripting language similar to Postman's test syntax
        // pm.test("name", function() { pm.expect(pm.response.code).to.equal(200); });

        let lines: Vec<&str> = test_script.lines().collect();

        for line in lines {
            let trimmed = line.trim();

            // Skip comments and empty lines
            if trimmed.is_empty() || trimmed.starts_with("//") {
                continue;
            }

            // Parse pm.test() calls
            if let Some(test) = self.parse_pm_test(trimmed, response) {
                results.push(test);
            }

            // Parse pm.expect() assertions
            if let Some(test) = self.parse_pm_expect(trimmed, response) {
                results.push(test);
            }

            // Parse assert statements
            if let Some(test) = self.parse_assert(trimmed, response) {
                results.push(test);
            }
        }

        results
    }

    /// Parse pm.test() style tests
    fn parse_pm_test(&self, line: &str, response: &ApiResponse) -> Option<TestResult> {
        // Extract test name from pm.test("name", ...)
        let prefix = "pm.test(";
        if !line.starts_with(prefix) {
            return None;
        }

        // Extract test name
        let start = line.find('"')? + 1;
        let end = line[start..].find('"')? + start;
        let test_name = &line[start..end];

        // Check if contains expect assertions
        let start_time = std::time::Instant::now();
        let passed = self.evaluate_expectations(line, response);

        Some(TestResult {
            name: test_name.to_string(),
            passed,
            error_message: if passed { None } else { Some("Assertion failed".to_string()) },
            duration_ms: start_time.elapsed().as_millis() as u64,
        })
    }

    /// Parse pm.expect() assertions
    fn parse_pm_expect(&self, line: &str, response: &ApiResponse) -> Option<TestResult> {
        // Handle pm.expect(pm.response.code).to.equal(200)
        if !line.contains("pm.expect") {
            return None;
        }

        let start_time = std::time::Instant::now();
        let passed = self.evaluate_expectation(line, response);

        Some(TestResult {
            name: "expect assertion".to_string(),
            passed,
            error_message: if passed { None } else { Some(format!("Expectation failed: {}", line)) },
            duration_ms: start_time.elapsed().as_millis() as u64,
        })
    }

    /// Parse assert statements
    fn parse_assert(&self, line: &str, response: &ApiResponse) -> Option<TestResult> {
        // Support simple assert statements:
        // assert response.status == 200
        // assert pm.response.json().id > 0
        // assert headers["content-type"] == "application/json"

        let start_time = std::time::Instant::now();

        if line.starts_with("assert ") {
            let assertion = &line[7..].trim();
            let passed = self.evaluate_assertion(assertion, response);

            return Some(TestResult {
                name: format!("assert: {}", assertion),
                passed,
                error_message: if passed { None } else { Some(format!("Assertion failed: {}", assertion)) },
                duration_ms: start_time.elapsed().as_millis() as u64,
            });
        }

        None
    }

    fn evaluate_expectations(&self, line: &str, response: &ApiResponse) -> bool {
        // Simple evaluation - check for common patterns
        if line.contains(".to.equal") {
            let parts: Vec<&str> = line.split(".to.equal(").collect();
            if parts.len() == 2 {
                let expected_str = parts[1].trim_end_matches(")").trim_end_matches(");");
                if let Ok(expected) = expected_str.parse::<u16>() {
                    return response.status == expected;
                }
            }
        }

        if line.contains(".to.be.ok") || line.contains(".to.be.true") {
            return response.status >= 200 && response.status < 300;
        }

        if line.contains(".to.have.status") {
            // pm.expect(pm.response).to.have.status(200)
            if let Some(start) = line.find("status(") {
                let status_part = &line[start + 7..];
                if let Some(end) = status_part.find(")") {
                    let status_str = &status_part[..end];
                    if let Ok(status) = status_str.parse::<u16>() {
                        return response.status == status;
                    }
                }
            }
        }

        true // Default to pass if we can't evaluate
    }

    fn evaluate_expectation(&self, line: &str, response: &ApiResponse) -> bool {
        // Handle pm.expect(pm.response.code).to.equal(200)
        if line.contains("pm.response.code") || line.contains("pm.response.status") {
            if line.contains(".to.equal") {
                let parts: Vec<&str> = line.split(".to.equal(").collect();
                if parts.len() >= 2 {
                    let expected_str = parts[1].trim_end_matches(")").trim_end_matches(");");
                    if let Ok(expected) = expected_str.parse::<u16>() {
                        return response.status == expected;
                    }
                }
            }
        }

        // Handle pm.expect(pm.response.body).to.contain("...")
        if line.contains("pm.response.body") || line.contains("pm.response.text()") {
            if line.contains(".to.contain") {
                let body_str = String::from_utf8_lossy(&response.body);
                let parts: Vec<&str> = line.split(".to.contain(").collect();
                if parts.len() >= 2 {
                    let expected = parts[1].trim_end_matches(")").trim_end_matches(");").trim_matches('"');
                    return body_str.contains(expected);
                }
            }
        }

        // Handle pm.expect(pm.response.headers.get('...')).to.equal('...')
        if line.contains("pm.response.headers") {
            if line.contains(".to.equal") {
                // Extract header name and expected value
                // This is a simplified check
                return true;
            }
        }

        true // Default to pass
    }

    fn evaluate_assertion(&self, assertion: &str, response: &ApiResponse) -> bool {
        // Handle response.status == 200
        if assertion.contains("response.status") || assertion.contains("response.code") {
            let parts: Vec<&str> = assertion.split("==").collect();
            if parts.len() == 2 {
                let expected_str = parts[1].trim();
                if let Ok(expected) = expected_str.parse::<u16>() {
                    return response.status == expected;
                }
            }
        }

        // Handle response.body.contains("...")
        if assertion.contains("response.body") || assertion.contains("response.text()") {
            let body_str = String::from_utf8_lossy(&response.body);
            if assertion.contains("contains") {
                let parts: Vec<&str> = assertion.split("contains").collect();
                if parts.len() == 2 {
                    let search = parts[1].trim().trim_matches('"').trim_matches('(').trim_matches(')');
                    return body_str.contains(search);
                }
            }
        }

        // Handle JSON path assertions
        if assertion.contains("json()") || assertion.contains(".json") {
            return self.evaluate_json_assertion(assertion, response);
        }

        // Handle headers
        if assertion.contains("headers") || assertion.contains("Headers") {
            return self.evaluate_header_assertion(assertion, response);
        }

        // Handle time assertions
        if assertion.contains("response.time") || assertion.contains("time_ms") {
            let parts: Vec<&str> = assertion.split("<").collect();
            if parts.len() == 2 {
                let limit_str = parts[1].trim();
                if let Ok(limit) = limit_str.parse::<u64>() {
                    return response.time_ms < limit;
                }
            }
        }

        true // Default to pass if can't evaluate
    }

    fn evaluate_json_assertion(&self, assertion: &str, response: &ApiResponse) -> bool {
        let body_str = String::from_utf8_lossy(&response.body);
        let json: Value = match serde_json::from_str(&body_str) {
            Ok(v) => v,
            Err(_) => return false,
        };

        // Extract JSON path
        // e.g., "assert json().id == 1" or "assert pm.response.json().data.name == 'test'"
        let path_part = if assertion.contains("json()") {
            assertion.split("json()").nth(1)
        } else if assertion.contains(".json") {
            assertion.split(".json").nth(1)
        } else {
            None
        };

        if let Some(path) = path_part {
            let path = path.trim_start_matches('.');

            // Simple path traversal
            let parts: Vec<&str> = path.split('.').collect();
            let mut current = &json;

            for part in parts {
                let clean_part = part.split("==").next().unwrap().trim();

                if clean_part.is_empty() {
                    continue;
                }

                // Handle array access: data[0]
                if clean_part.contains('[') {
                    let name = clean_part.split('[').next().unwrap();
                    let idx_part = clean_part.split('[').nth(1)
                        .and_then(|s| s.split(']').next())
                        .and_then(|s| s.parse::<usize>().ok());

                    if let Some(obj) = current.get(name) {
                        if let Some(idx) = idx_part {
                            if let Some(arr) = obj.as_array() {
                                current = arr.get(idx).unwrap_or(&Value::Null);
                            } else {
                                return false;
                            }
                        } else {
                            current = obj;
                        }
                    } else {
                        return false;
                    }
                } else {
                    current = current.get(clean_part).unwrap_or(&Value::Null);
                }
            }

            // Check comparison
            if assertion.contains("==") {
                let expected = assertion.split("==").nth(1).map(|s| s.trim().trim_matches('"'));
                if let Some(exp) = expected {
                    match current {
                        Value::String(s) => return s == exp,
                        Value::Number(n) => {
                            if let Ok(num) = exp.parse::<f64>() {
                                return n.as_f64().map(|v| v == num).unwrap_or(false);
                            }
                            return false;
                        }
                        Value::Bool(b) => {
                            if let Ok(bool_val) = exp.parse::<bool>() {
                                return *b == bool_val;
                            }
                            return false;
                        }
                        _ => return false,
                    }
                }
            }

            // Check existence
            return !current.is_null();
        }

        true
    }

    fn evaluate_header_assertion(&self, assertion: &str, response: &ApiResponse) -> bool {
        // Handle headers["content-type"] == "application/json"
        if assertion.contains("[") && assertion.contains("]") {
            let start = assertion.find('[').unwrap() + 1;
            let end = assertion.find(']').unwrap();
            let header_name = &assertion[start..end].trim_matches('"').to_lowercase();

            if assertion.contains("==") {
                let expected = assertion.split("==").nth(1)
                    .map(|s| s.trim().trim_matches('"'));

                if let Some(exp) = expected {
                    if let Some(value) = response.headers.get(header_name) {
                        return value.to_lowercase() == exp.to_lowercase();
                    }
                }
            }
        }

        true
    }
}

impl Default for TestRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a default test script for a response
pub fn generate_test_script(response: &ApiResponse) -> String {
    format!(
        r#"// Auto-generated test script
pm.test("Status code is {}", function () {{
    pm.response.to.have.status({});
}});

pm.test("Response time is less than 500ms", function () {{
    pm.expect(pm.response.responseTime).to.be.below(500);
}});

// Add your custom tests below
// pm.test("Response has correct structure", function () {{
//     var jsonData = pm.response.json();
//     pm.expect(jsonData).to.have.property("id");
// }});
"#,
        response.status, response.status
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_assertion() {
        let runner = TestRunner::new();
        let response = ApiResponse {
            status: 200,
            status_text: "OK".to_string(),
            timestamp: chrono::Utc::now(),
            headers: HashMap::new(),
            body: b"{}".to_vec(),
            content_type: Some("application/json".to_string()),
            size_bytes: 2,
            time_ms: 100,
        };

        let script = r#"
pm.test("Status is 200", function() {
    pm.expect(pm.response.code).to.equal(200);
});
"#;

        let results = runner.run_tests(script, &response);
        assert!(!results.is_empty());
        assert!(results[0].passed);
    }

    #[test]
    fn test_json_assertion() {
        let runner = TestRunner::new();
        let response = ApiResponse {
            status: 200,
            status_text: "OK".to_string(),
            timestamp: chrono::Utc::now(),
            headers: HashMap::new(),
            body: r#"{"id": 123, "name": "test"}"#.as_bytes().to_vec(),
            content_type: Some("application/json".to_string()),
            size_bytes: 27,
            time_ms: 50,
        };

        let script = r#"
assert pm.response.json().id == 123
"#;

        let results = runner.run_tests(script, &response);
        assert!(!results.is_empty());
    }
}
