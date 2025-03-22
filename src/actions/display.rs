use crate::db::item::Item;

// For quick debug purposes
pub fn debug_print_items(header: &str, items: &[Item]) {
    println!("{}", header);
    for item in items {
        println!("  {:?}", item);
    }
}
