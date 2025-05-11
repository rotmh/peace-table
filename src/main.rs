use peace_table::PieceTable;

fn main() {
    const CH: &str = "a";
    let mut pt = PieceTable::new("asdfjlkajslkdfjlkajsldkfjlkasjdlkfj");
    for i in 10..10000 {
        pt.insert(i, CH);
    }
    pt.insert(2, CH);
    pt.remove(4..294);
    for i in 3..5531 {
        pt.insert(i, CH);
    }
}
