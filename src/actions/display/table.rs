use std::cmp;

use terminal_size::{
    terminal_size,
    Width,
};
use textwrap::{
    wrap,
    Options,
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

    let index_options = Options::new(index_width).break_words(true);
    let category_options = Options::new(category_width).break_words(true);
    let content_options = Options::new(content_width).break_words(false);
    let timestr_options = Options::new(timestr_width).break_words(false);

    for row in rows {
        let wrapped_index = wrap(&row.index, &index_options);
        let wrapped_category = wrap(&row.category, &category_options);
        let wrapped_content = wrap(&row.content, &content_options);
        let wrapped_timestr = wrap(&row.timestr, &timestr_options);

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
                "| {:<index_width$}| {:<category_width$}| {:<content_width$}| {:<timestr_width$}|",
                index_line,
                category_line,
                content_line,
                timestr_line,
                index_width = index_width,
                category_width = category_width,
                content_width = content_width,
                timestr_width = timestr_width
            );
        }

        // Print separator between rows
        println!("{:-<width$}", "", width = separator_width);
    }
}
