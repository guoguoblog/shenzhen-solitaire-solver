pub fn join_vertical(strings: Vec<String>) -> String {
    let mut result = String::new();
    let columns: Vec<Vec<_>> = strings.iter().map(|str| str.split("\n").collect()).collect();
    let length = columns.iter().map(|strs| strs.len()).max().expect("input must not be empty");

    for y in 0..length {
        // TODO: doesn't fit this util really, but it's certainly the easiest place to add it
        result.push_str(" ");
        for column in columns.iter() {
            result.push_str(column.get(y).unwrap_or(&" "));
        }
        result.push_str("\n");
    }

    return result;
}
