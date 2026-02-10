use crate::{
    actions::display::{
        print_table,
        DisplayRow,
    },
    db::item::Item,
};

// For quick debug purposes
#[allow(dead_code)]
pub fn debug_print_items(header: &str, items: &[Item]) {
    println!("{}", header);
    for item in items {
        println!("  {:?}", item);
    }
}

pub fn print_bold(text: &str) {
    println!("\x1b[1m{}\x1b[0m", text);
}

pub fn print_red(text: &str) {
    println!("\x1b[91m{}\x1b[0m", text);
}

// print items in a table.
pub fn print_items(items: &[Item], is_record: bool, is_list: bool) {
    let mut results: Vec<DisplayRow> = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let indexstr = if is_list {
            format!("{}", index + 1)
        } else {
            "N/A".to_string()
        };
        // Check each item's actual type instead of using a global boolean
        let item_is_record = item.action == "record" || item.action == "recurring_task_record";
        if item_is_record {
            results.push(DisplayRow::from_record(indexstr, item));
        } else {
            results.push(DisplayRow::from_task(indexstr, item))
        }
    }
    print_table(&results, is_record);
}
