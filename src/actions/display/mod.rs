mod print;
mod row;
mod table;

pub use crate::actions::display::{
    print::{
        print_bold,
        print_items,
        print_red,
    },
    row::DisplayRow,
    table::print_table,
};
