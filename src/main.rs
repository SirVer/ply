#[macro_use]
extern crate nom;

use nom::{IResult, alpha, space, not_line_ending, digit, alphanumeric, multispace, GetOutput};
use std::fs::File;
use std::io::prelude::*;
use std::str::from_utf8;

#[derive(Debug, PartialEq, Eq)]
struct Version {
    major: i32,
    minor: i32,
}

#[derive(Debug, PartialEq, Eq)]
enum FormatKind {
    Ascii,
    BigEndian,
    LittleEndian,
}

#[derive(Debug, PartialEq, Eq)]
struct Format {
    kind: FormatKind,
    version: Version,
}

#[derive(Debug, PartialEq, Eq)]
struct Header {
    comments: Vec<String>,
    format: Format,
    elements: Vec<Element>,
}

named!(format_version<Version>,
   chain!(
       major: map_res!(map_res!(digit, from_utf8), str::parse) ~
       tag!(".") ~
       minor: map_res!(map_res!(digit,  from_utf8), str::parse),
       || Version { minor: minor, major: major }
    )
);



named!(format<Format>,
   chain!(
       tag!("format") ~
       multispace ~
       kind: alt!(
           map!(tag!("ascii"), |_| FormatKind::Ascii) |
           map!(tag!("binary_big_endian"), |_| FormatKind::BigEndian) |
           map!(tag!("binary_little_endian"), |_| FormatKind::LittleEndian)
       ) ~
       multispace ~
       version: format_version ~
       multispace,
       || Format {
           kind: kind,
           version: version,
       }
    )
);


#[derive(Debug, PartialEq, Eq)]
enum DataType {
    Int8,
    UInt8,
    Int16,
    UInt16,
    Int32,
    UInt32,
    Int64,
    UInt64,
    Float32,
    Float64,
}

#[derive(Debug, PartialEq, Eq)]
enum PropertyKind {
    Scalar(DataType),
    List(DataType, DataType),
}

#[derive(Debug, PartialEq, Eq)]
struct Property {
    name: String,
    kind: PropertyKind,
}

#[derive(Debug, PartialEq, Eq)]
struct Element {
    name: String,
    count: i64,
    properties: Vec<Property>,
}


fn is_identifier(a: u8) -> bool {
    match a as char {
        'a' ... 'z' => true,
        'A' ... 'Z' => true,
        '_' => true,
        _ => false,
    }
}

named!(identifier<&[u8]>,
    take_while1!(is_identifier)
);

named!(data_type<DataType>,
   alt!(
       map!(tag!("char"), |_| DataType::Int8) |
       map!(tag!("uchar"), |_| DataType::UInt8) |

       map!(tag!("short"), |_| DataType::Int16) |
       map!(tag!("ushort"), |_| DataType::UInt16) |

       map!(tag!("int64"), |_| DataType::Int64) |
       map!(tag!("int32"), |_| DataType::Int32) |
       map!(tag!("int16"), |_| DataType::Int16) |
       map!(tag!("int8"), |_| DataType::Int8) |
       map!(tag!("int"), |_| DataType::Int32) |

       map!(tag!("uint8"), |_| DataType::UInt8) |
       map!(tag!("uint16"), |_| DataType::UInt16) |
       map!(tag!("uint32"), |_| DataType::UInt32) |
       map!(tag!("uint64"), |_| DataType::UInt64) |
       map!(tag!("uint"), |_| DataType::UInt32) |

       map!(tag!("float32"), |_| DataType::Float32) |
       map!(tag!("float64"), |_| DataType::Float64) |
       map!(tag!("float"), |_| DataType::Float32) |

       map!(tag!("double"), |_| DataType::Float64)
   )
);

named!(property<Property>,
    chain!(
        tag!("property") ~
        multispace ~
        kind: alt!(
            chain!(
                tag!("list") ~
                multispace ~
                count_data_type: data_type ~
                multispace ~
                element_data_type: data_type,
                || PropertyKind::List(count_data_type, element_data_type)
            ) |
            map!(data_type, |d| PropertyKind::Scalar(d))
        ) ~
        multispace ~
        name: map_res!(identifier, from_utf8) ~
        multispace,
        || Property {
            name: name.to_string(),
            kind: kind,
        }
    )
);

named!(element<Element>,
    chain!(
        tag!("element") ~
        multispace ~
        name: map_res!(identifier, from_utf8) ~
        multispace ~
        count: map_res!(map_res!(digit, from_utf8), str::parse) ~
        multispace ~
        properties: many1!(property),
        || Element {
            name: name.to_string(),
            count: count,
            properties: properties,
        }
    )
);

named!(comment<String>,
    chain!(
        tag!("comment") ~
        multispace ~
        comment: map_res!(not_line_ending, from_utf8) ~
        multispace,
        || comment.to_string()
    )
);

named!(header<&[u8], Header>,
    chain!(
        tag!("ply") ~
        multispace ~
        format: format ~
        comments: many0!(comment) ~
        elements: many1!(element) ~
        tag!("end_header") ~
        multispace,
        || {
            Header {
                comments: comments,
                format: format,
                elements: elements,
            }
        }
    )
);

#[test]
fn parse_category_test() {
    let input = b"property list uint8 int32 vertex_indices\n";
    let res = property(input);
    if let IResult::Done(_, res) = res {
        assert_eq!(Property {
            kind: PropertyKind::List(DataType::UInt8, DataType::Int32),
            name: "vertex_indices".into(),
        }, res);
    } else {
        panic!("res: {:?}", res);
    }
}


fn main() {
    let mut v = Vec::new();
    File::open("testdata/beethoven.ply")
        .unwrap()
        .read_to_end(&mut v)
        .unwrap();
    match header(&v) {
        IResult::Done(_, ply) => {
            println!("#sirver ply: {:#?}", ply);
        }
        IResult::Error(err) => panic!("Error: {:?}", err),
        IResult::Incomplete(a) => {
            println!("#sirver a: {:#?}", a);
        },
    }
}
