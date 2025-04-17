mod print;
mod row;
mod table;

pub use crate::actions::display::{
    print::{
        print_bold,
        print_items,
    },
    row::DisplayRow,
    table::print_table,
};
