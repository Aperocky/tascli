use std::cmp;

use terminal_size::{
    terminal_size,
    Width,
};
use unicode_width::{
    UnicodeWidthChar,
    UnicodeWidthStr,
};

use crate::actions::display::DisplayRow;

pub fn print_table(rows: &[DisplayRow], is_record: bool) {
    let terminal_width = if let Some((Width(w), _)) = terminal_size() {
        w as usize
    } else {
        120 // Default if unable to detect
    };

    // Define column widths
    let index_width = 7;
    let category_width = 20;
    let timestr_width = 20;
    let margin = 10;

    // Calculate content width
    // Total used: column widths + 5 delimiters (|) + margin
    let content_width =
        terminal_width.saturating_sub(index_width + category_width + timestr_width + 5 + margin);

    let time_header = if is_record { "Created At" } else { "Deadline" };

    let separator_width = terminal_width - margin + 4;

    // Print table header
    println!("{:-<width$}", "", width = separator_width);
    println!(
        "| {:<index_width$}| {:<category_width$}| {:<content_width$}| {:<timestr_width$}|",
        "Index",
        "Category",
        "Content",
        time_header,
        index_width = index_width,
        category_width = category_width,
        content_width = content_width,
        timestr_width = timestr_width
    );
    println!("{:-<width$}", "", width = separator_width);

    for row in rows {
        let wrapped_index = wrap(&row.index, index_width);
        let wrapped_category = wrap(&row.category, category_width);
        let wrapped_content = wrap(&row.content, content_width);
        let wrapped_timestr = wrap(&row.timestr, timestr_width);

        // Find the maximum number of lines needed
        let max_lines = cmp::max(
            cmp::max(wrapped_index.len(), wrapped_category.len()),
            cmp::max(wrapped_content.len(), wrapped_timestr.len()),
        );

        for i in 0..max_lines {
            let index_line = if i < wrapped_index.len() {
                &wrapped_index[i]
            } else {
                ""
            };
            let category_line = if i < wrapped_category.len() {
                &wrapped_category[i]
            } else {
                ""
            };
            let content_line = if i < wrapped_content.len() {
                &wrapped_content[i]
            } else {
                ""
            };
            let timestr_line = if i < wrapped_timestr.len() {
                &wrapped_timestr[i]
            } else {
                ""
            };

            println!(
                "| {}| {}| {}| {}|",
                pad_string(index_line, index_width),
                pad_string(category_line, category_width),
                pad_string(content_line, content_width),
                pad_string(timestr_line, timestr_width)
            );
        }

        // Print separator between rows
        println!("{:-<width$}", "", width = separator_width);
    }
}

fn pad_string(s: &str, width: usize) -> String {
    let term_width = UnicodeWidthStr::width(s);
    if term_width >= width {
        s.to_string()
    } else {
        let mut res = s.to_string();
        res.push_str(&" ".repeat(width - term_width));
        res
    }
}

// Wraps text with consideration for unicode characters and word continuity.
fn wrap(text: &str, max_length: usize) -> Vec<String> {
    if max_length == 0 || text.is_empty() {
        return vec![];
    }

    let mut result = Vec::new();
    let mut current_line = String::new();

    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\n' {
            result.push(current_line.trim().to_string());
            current_line = String::new();
            continue;
        }
        if c.is_whitespace() && current_line.is_empty() {
            continue;
        }

        if c.is_ascii_alphanumeric() || c == '_' {
            // For alphanumeric words, we wrap without breaking words
            // Create a word of continuous alphanumeric character
            let mut word = String::new();
            word.push(c);
            while let Some(&next_c) = chars.peek() {
                if !next_c.is_ascii_alphanumeric() || c == '_' {
                    break;
                }
                word.push(chars.next().unwrap());
            }

            if current_line.len() + word.len() <= max_length {
                // word fit on currentline, add it.
                current_line.push_str(&word);
            } else if word.len() <= max_length {
                // word doesn't fit, but can fit on a new line
                if !current_line.is_empty() {
                    result.push(current_line.trim().to_string());
                    current_line = word;
                } else {
                    current_line = word;
                }
            } else {
                // Word is too long, must be broken
                if !current_line.is_empty() {
                    result.push(current_line.trim().to_string());
                    current_line = String::new();
                }
                for char in word.chars() {
                    if current_line.len() + 2 <= max_length {
                        current_line.push(char);
                    } else {
                        current_line.push('-');
                        result.push(current_line.trim().to_string());
                        current_line = String::new();
                        current_line.push(char);
                    }
                }
            }
        } else {
            // For non-ASCII alphabetic characters or whitespace
            if current_line.len() + UnicodeWidthChar::width(c).unwrap_or(1) <= max_length {
                current_line.push(c);
            } else {
                result.push(current_line.trim().to_string());
                current_line = String::new();
                if !c.is_whitespace() {
                    current_line.push(c);
                }
            }
        }
    }

    if !current_line.is_empty() {
        result.push(current_line.trim().to_string());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_function() {
        let test_cases = vec![
            // (input, max_width, expected_output)
            ("", 10, vec![]),
            ("Hello", 10, vec!["Hello"]),
            ("Hello", 5, vec!["Hello"]),
            ("Hello world", 5, vec!["Hello", "world"]),
            (
                "Supercalifragilisticexpialidocious",
                10,
                vec!["Supercali-", "fragilist-", "icexpiali-", "docious"],
            ),
            (
                "This is a test with some-hyphenated words.",
                10,
                vec!["This is a", "test with", "some-", "hyphenated", "words."],
            ),
            ("Hello\nworld", 10, vec!["Hello", "world"]),
            ("user_name is_valid", 10, vec!["user_name", "is_valid"]),
            ("你好世界", 5, vec!["你好", "世界"]),
        ];

        for (input, max_width, expected) in test_cases {
            assert_eq!(
                wrap(input, max_width),
                expected,
                "Failed on input: '{}' with max_width: {}",
                input,
                max_width
            );
        }
    }
}
