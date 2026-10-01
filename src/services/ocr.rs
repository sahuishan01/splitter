use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    pub raw_text: String,
    pub detected_amount_cents: Option<i64>,
    pub detected_amount: Option<f64>,
    pub detected_merchant: Option<String>,
}

/// Execute Tesseract OCR on a local image file and parse receipt data.
pub async fn run_ocr_on_file(file_path: &Path) -> anyhow::Result<OcrResult> {
    let output = tokio::process::Command::new("tesseract")
        .arg(file_path)
        .arg("stdout")
        .arg("-l")
        .arg("eng")
        .output()
        .await?;

    let raw_text = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(parse_receipt_text(&raw_text))
}

/// Parse receipt/bill OCR text to extract total amount and merchant name.
pub fn parse_receipt_text(text: &str) -> OcrResult {
    let raw_text = text.to_string();
    let detected_merchant = extract_merchant(text);
    let detected_amount_cents = extract_final_amount(text);
    let detected_amount = detected_amount_cents.map(|c| c as f64 / 100.0);

    OcrResult {
        raw_text,
        detected_amount_cents,
        detected_amount,
        detected_merchant,
    }
}

/// Extract store/merchant name from receipt header lines
fn extract_merchant(text: &str) -> Option<String> {
    let non_merchant_patterns = [
        "tax invoice", "invoice", "receipt", "sales receipt", "bill",
        "cash receipt", "welcome", "customer copy", "store copy", "duplicate",
        "order #", "table #", "terminal", "date:", "time:", "cashier:", "tel:",
        "phone:", "http:", "https:", "www."
    ];

    for line in text.lines().take(6) {
        let trimmed = line.trim();
        if trimmed.len() < 2 || trimmed.len() > 50 {
            continue;
        }

        let lower = trimmed.to_lowercase();
        let is_skip = non_merchant_patterns.iter().any(|p| lower.contains(p))
            || trimmed.chars().all(|c| c.is_ascii_digit() || c.is_ascii_punctuation())
            || lower.starts_with('#');

        if !is_skip {
            // Clean common trailing punctuation or symbols
            let clean = trimmed.trim_matches(|c: char| !c.is_alphanumeric()).to_string();
            if clean.len() >= 3 {
                return Some(clean);
            }
        }
    }

    None
}

/// Extract final payable amount in cents from bill/receipt text
fn extract_final_amount(text: &str) -> Option<i64> {
    let lines: Vec<&str> = text.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    if lines.is_empty() {
        return None;
    }

    // High confidence keyword regexes (Total, Grand Total, Net Payable, Amount Due, etc.)
    // Explicitly avoids 'subtotal', 'tax', 'cash', 'change'
    let strong_keywords = [
        r"(?i)\bgrand\s*total\b",
        r"(?i)\bnet\s*(?:payable|amount|due)\b",
        r"(?i)\bamount\s*due\b",
        r"(?i)\bbalance\s*due\b",
        r"(?i)\btotal\s*payable\b",
        r"(?i)\btotal\s*amount\b",
        r"(?i)\bfinal\s*total\b",
    ];

    let standard_total = r"(?i)\b(?:total|totale|tot)\b";

    // Regex to extract numbers like 45.99, $1,234.50, 45,00
    let amount_regex = Regex::new(r"[$€£₹¥]?\s*([0-9]{1,3}(?:[,\s][0-9]{3})*(?:\.[0-9]{1,2})|[0-9]+(?:\.[0-9]{1,2})|[0-9]+,[0-9]{2})").unwrap();

    // 1. Check strong keywords from bottom to top (bottom-up priority)
    for strong_kw in &strong_keywords {
        let kw_re = Regex::new(strong_kw).unwrap();
        for line in lines.iter().rev() {
            if kw_re.is_match(line) {
                if let Some(cents) = parse_line_amount(line, &amount_regex) {
                    return Some(cents);
                }
            }
        }
    }

    // 2. Check standard 'total' lines from bottom to top, excluding subtotals/change/cash
    let total_re = Regex::new(standard_total).unwrap();
    for (idx, line) in lines.iter().enumerate().rev() {
        let lower = line.to_lowercase();
        if total_re.is_match(line) {
            // Exclude lines with subtotal, tax, cash, change, items
            let is_subtotal = lower.contains("sub") || lower.contains("sub-total");
            let is_tax = lower.contains("tax") || lower.contains("vat") || lower.contains("gst");
            let is_change = lower.contains("change") || lower.contains("tendered") || lower.contains("cash");
            let is_count = lower.contains("items") || lower.contains("qty") || lower.contains("count");

            if !is_subtotal && !is_tax && !is_change && !is_count {
                if let Some(cents) = parse_line_amount(line, &amount_regex) {
                    return Some(cents);
                }
                // Sometimes the amount is on the next line
                if idx + 1 < lines.len() {
                    if let Some(cents) = parse_line_amount(lines[idx + 1], &amount_regex) {
                        return Some(cents);
                    }
                }
            }
        }
    }

    // 3. Fallback: Check for currency symbol lines in bottom 50% of the receipt
    let start_idx = lines.len() / 2;
    let mut candidate_amounts: Vec<i64> = Vec::new();
    for line in &lines[start_idx..] {
        let lower = line.to_lowercase();
        if lower.contains("change") || lower.contains("cash") || lower.contains("card") || lower.contains("visa") || lower.contains("mc") {
            continue;
        }
        for cap in amount_regex.captures_iter(line) {
            if let Some(num_match) = cap.get(1) {
                if let Some(cents) = string_to_cents(num_match.as_str()) {
                    // Plausible receipt total between $0.50 and $50,000.00
                    if cents >= 50 && cents <= 5_000_000 {
                        candidate_amounts.push(cents);
                    }
                }
            }
        }
    }

    // If candidate amounts exist, pick the maximum from the bottom section (usually the total)
    if let Some(&max_cents) = candidate_amounts.iter().max() {
        return Some(max_cents);
    }

    None
}

/// Helper to parse amount from a single line string
fn parse_line_amount(line: &str, regex: &Regex) -> Option<i64> {
    // Scan all matching numbers on this line and pick the last one (amounts are usually at the end of the line)
    let mut matches = Vec::new();
    for cap in regex.captures_iter(line) {
        if let Some(m) = cap.get(1) {
            if let Some(cents) = string_to_cents(m.as_str()) {
                if cents > 0 {
                    matches.push(cents);
                }
            }
        }
    }
    matches.pop()
}

/// Convert number string e.g. "45.99", "1,234.50", "45,00" to cents (integer)
fn string_to_cents(val_str: &str) -> Option<i64> {
    let cleaned = val_str.replace(' ', "").replace(',', ".");
    // If multiple dots, e.g. 1.234.50, handle thousand separators
    let dot_count = cleaned.matches('.').count();
    let standardized = if dot_count > 1 {
        // e.g. "1.234.50" -> "1234.50"
        let parts: Vec<&str> = cleaned.split('.').collect();
        let decimal = parts.last().unwrap_or(&"00");
        let whole = parts[..parts.len() - 1].join("");
        format!("{}.{}", whole, decimal)
    } else {
        cleaned
    };

    let val: f64 = standardized.parse().ok()?;
    if val <= 0.0 || val > 1_000_000_000.0 {
        return None;
    }
    Some((val * 100.0).round() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_receipt_parsing() {
        let receipt = r#"
            STARBUCKS COFFEE #1042
            123 MAIN STREET
            Order: 4921 Date: 2026-09-20
            1 Caramel Macchiato    $5.45
            1 Croissant            $3.75
            Subtotal               $9.20
            Tax                    $0.74
            TOTAL                  $9.94
            CASH                   $20.00
            CHANGE                 $10.06
            Thank you for visiting!
        "#;

        let result = parse_receipt_text(receipt);
        assert_eq!(result.detected_merchant.as_deref(), Some("STARBUCKS COFFEE #1042"));
        assert_eq!(result.detected_amount_cents, Some(994));
        assert_eq!(result.detected_amount, Some(9.94));
    }

    #[test]
    fn test_grand_total_parsing() {
        let receipt = r#"
            WHOLE FOODS MARKET
            GROCERY ITEMS
            Item 1          12.50
            Item 2          35.00
            SUBTOTAL        47.50
            TAX              3.80
            GRAND TOTAL: $51.30
        "#;

        let result = parse_receipt_text(receipt);
        assert_eq!(result.detected_merchant.as_deref(), Some("WHOLE FOODS MARKET"));
        assert_eq!(result.detected_amount_cents, Some(5130));
    }

    #[test]
    fn test_net_payable_receipt() {
        let receipt = r#"
            TRADER JOE'S
            INVOICE #4421
            NET PAYABLE: 125.75
            PAID BY CARD: 125.75
        "#;

        let result = parse_receipt_text(receipt);
        assert_eq!(result.detected_merchant.as_deref(), Some("TRADER JOE'S"));
        assert_eq!(result.detected_amount_cents, Some(12575));
    }

    #[test]
    fn test_multiline_total_parsing() {
        let receipt = r#"
            CAFE BISTRO
            Coffee     4.00
            TOTAL DUE
            $4.00
        "#;

        let result = parse_receipt_text(receipt);
        assert_eq!(result.detected_amount_cents, Some(400));
    }
}
