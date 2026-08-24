use crate::tool::direction_parse::parse_expr_to_dir;
use crate::tool::parse_to_dir;
#[test]
fn test_parse_to_dir() {
    let inputs = vec![
        "TO X (NEG ( 20 )) Y ( 40 ) Z ( 65 )",
        // "TO X NEG(20) Y 30 Z 65",
        // "TO X NEG 20.7 Y 45.1 Z NEG 10.2",
        "TO X (10.5) ",
    ];

    for input in inputs {
        let r = parse_expr_to_dir(input);
        dbg!(r);
        // match parse_to_dir::parse_to_direction(input) {
        //     Ok((_, direction)) => println!("Input: '{}'\nParsed direction: {:?}\n", input, direction),
        //     Err(e) => println!("Error parsing direction '{}': {:?}\n", input, e),
        // }
    }
}

// #[test]
// fn test_parse_dir() {
//     let str = "TO X (NEG ( 20 )) Z ( 65 )";
//     let str = "X 30 Y";
//     let v = parse_expr_to_dir(str).unwrap();
//     dbg!(v);
// }
