#[macro_use]
extern crate nom;

use nom::{IResult, is_space, alpha, space, not_line_ending, digit, alphanumeric, multispace,
          GetOutput};
use std::fs::File;
use std::io::prelude::*;
use std::str::from_utf8;
use std::str::FromStr;

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
enum ValueKind {
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

#[derive(Debug)]
enum Value {
    Int8(i8),
    UInt8(u8),
    Int16(i16),
    UInt16(u16),
    Int32(i32),
    UInt32(u32),
    Int64(i64),
    UInt64(u64),
    Float32(f32),
    Float64(f64),
}

#[derive(Debug, PartialEq, Eq)]
enum PropertyKind {
    Scalar(ValueKind),
    List(ValueKind, ValueKind),
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
        'a'...'z' => true,
        'A'...'Z' => true,
        '_' => true,
        _ => false,
    }
}

named!(identifier<&[u8]>,
    take_while1!(is_identifier)
);

named!(data_type<ValueKind>,
   alt!(
       map!(tag!("char"), |_| ValueKind::Int8) |
       map!(tag!("uchar"), |_| ValueKind::UInt8) |

       map!(tag!("short"), |_| ValueKind::Int16) |
       map!(tag!("ushort"), |_| ValueKind::UInt16) |

       map!(tag!("int64"), |_| ValueKind::Int64) |
       map!(tag!("int32"), |_| ValueKind::Int32) |
       map!(tag!("int16"), |_| ValueKind::Int16) |
       map!(tag!("int8"), |_| ValueKind::Int8) |
       map!(tag!("int"), |_| ValueKind::Int32) |

       map!(tag!("uint8"), |_| ValueKind::UInt8) |
       map!(tag!("uint16"), |_| ValueKind::UInt16) |
       map!(tag!("uint32"), |_| ValueKind::UInt32) |
       map!(tag!("uint64"), |_| ValueKind::UInt64) |
       map!(tag!("uint"), |_| ValueKind::UInt32) |

       map!(tag!("float32"), |_| ValueKind::Float32) |
       map!(tag!("float64"), |_| ValueKind::Float64) |
       map!(tag!("float"), |_| ValueKind::Float32) |

       map!(tag!("double"), |_| ValueKind::Float64)
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

fn ascii_value(input: &[u8], value_kind: ValueKind) -> IResult<&[u8], Value> {
    let token = chain!(input,
        token: map_res!(is_not!(b" \n"), from_utf8) ~
        multispace,
        || token
    );

    match token {
        IResult::Error(a) => IResult::Error(a),
        IResult::Incomplete(i) => IResult::Incomplete(i),
        IResult::Done(remaining, out) => {
            IResult::Done(remaining,
                          match value_kind {
                              ValueKind::Float32 => Value::Float32(f32::from_str(out).unwrap()),
                              _ => unimplemented!(),
                          })
        }
    }
}

fn value<'a>(input: &'a [u8],
             format_kind: &FormatKind,
             value_kind: ValueKind)
             -> IResult<&'a [u8], Value> {
    match *format_kind {
        FormatKind::Ascii => ascii_value(input, value_kind),
        FormatKind::LittleEndian | FormatKind::BigEndian => unimplemented!(),
    }
}

fn body<'a>(input: &'a [u8], header: &Header) -> IResult<&'a [u8], Value> {
    // NOCOM(#sirver): Assuming ASCII format for this discussion.
    for element in &header.elements[..1] { // NOCOM(#sirver): for debug reasons only use the first
        // The 'count' entry defines how many lines of property entries are coming now.
        for _ in 0..element.count {
            for property in &element.properties {
                // let y = value(input, &header.format.kind, ValueKind::Float32);
                // let y = value(input, &header.format.kind, ValueKind::Float32);
                println!("#sirver property: {:#?}", property);
            }
        }
    }
    // NOCOM(#sirver): this is only here to make the compiler happy
    value(input, &header.format.kind, ValueKind::Float32)
}

#[test]
fn parse_category_test() {
    let input = b"property list uint8 int32 vertex_indices\n";
    let res = property(input);
    if let IResult::Done(_, res) = res {
        assert_eq!(Property {
                       kind: PropertyKind::List(ValueKind::UInt8, ValueKind::Int32),
                       name: "vertex_indices".into(),
                   },
                   res);
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
        IResult::Done(remaining, header) => {
            println!("#sirver header: {:#?}", header);
            match body(remaining, &header) {
                IResult::Done(remaining, body) => {
                    println!("#sirver body: {:#?}", body);
                }
                IResult::Error(err) => panic!("Error: {:?}", err),
                IResult::Incomplete(a) => {
                    println!("#sirver a: {:#?}", a);
                }
            }
        }
        IResult::Error(err) => panic!("Error: {:?}", err),
        IResult::Incomplete(a) => {
            println!("#sirver a: {:#?}", a);
        }
    }
}
