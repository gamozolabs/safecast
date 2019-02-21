/// Procedural macro to validate the all members of a structure implement
/// `Safecast` and generates a runtime routine that can be used to validate
/// that no padding bytes are present
///
/// Since we implement `Safecast` only a plain-old-data root types
/// (eg. u8, u32, i32, etc) any structure composed entirely of `Safecast`
/// types will have no padding or non-POD. This allows us to safely do
/// byte-level copies and casts of the underlying data between types
/// implementing `Safecast`
///
/// Due to not being able to check sizes of values during a procedural macro
/// it's important to note that `Safecast::safecast()` must be invoked on a
/// type to do runtime checks of it's padding. Luckily these checks get
/// optimized out almost entirely in most cases as the compiler can constprop
/// the size calculations at compile time. We just can't do it for it here :(
///
/// Further this does not use `std` nor does it have third party dependencies
/// which allows for this codebase to be maximally portable.
///
/// Yes I'm aware that proc_macro2 and other crates exist and would help make
/// our lives easier here but I use this codebase everywhere and I really would
/// prefer to have zero dependencies.
///
/// Since we manually parse syntax here it's possible there are edge cases we
/// do not handle correctly (generics, where clauses, etc). But we can add
/// those as time goes on. Further you're not really working with templates
/// if you're working with POD anyways. So these might not really be needed
/// to implement anyways.

extern crate proc_macro;

use proc_macro::TokenStream;

#[proc_macro_derive(Safecast)]
pub fn derive_safecast(item: TokenStream) -> TokenStream {
    // Convert the `TokenStream` to a string
    // At this point the structure string representation will be normalized
    // and things like comments, unnecessary whitespace, etc will be removed.
    let stream = item.to_string();

    // Split up the structure definition into its lines
    let lines: Vec<&str> = stream.lines().collect();

    // Make sure the first line of this derived structure is a
    // `#[repr(C)]`
    assert!(lines.len() > 0 && lines[0] == "#[repr(C)]",
        "Safecast requires #[repr(C)]");

    // There has to be at least one line of the form:
    // Regular: `struct Moose {`
    // Tuple:   `struct Flat(u32, u32);`
    // Unit:    `struct Unit;`
    assert!(lines.len() > 1, "Malformed structure definition");

    // Make sure it's a struct
    assert!(lines[1].starts_with("struct "),
        "Type must be a struct for Safecast");

    // Figure out the type of this structure
    let is_tuple_struct = stream.ends_with(");");
    let is_named_struct = stream.ends_with("}");

    // Make sure it's either a named or tuple struct
    assert!((is_tuple_struct && !is_named_struct) ||
            (!is_tuple_struct && is_named_struct),
            "Unit structures not allowed in Safecast");

    // Now lets get the identifier
    let ident = if is_named_struct {
        lines[1].split("struct ").nth(1).unwrap().split(" {").nth(0).unwrap()
    } else {
        lines[1].split("struct ").nth(1).unwrap().split("(").nth(0).unwrap()
    };
    
    // Now we have to remove document comments. Normal comments `//` and
    // `/* */` were removed for us and thus will not be present, but there
    // will be `///` comments in the output. Let's remove them!
    // This also removes CRLFs from the input
    let mut commentless = String::new();
    for line in lines {
        if line.trim().starts_with("///") { continue; }
        commentless += line;
    }

    // Parse out the fields of the structure
    // Also remove all spaces, newlines, CRs, and tabs
    let fields = if is_named_struct {
        commentless.split(&format!("struct {} {{", ident)).nth(1)
            .expect("Could not find struct prefix")
            .split("}").nth(0).expect("Could not find struct postfix")
    } else {
        commentless.split(&format!("struct {}(", ident)).nth(1).unwrap()
            .split(");").nth(0).unwrap()
    }.replace(" ", "").replace("\t", "");

    // For a tuple struct fields should look like:
    // Fields: "u32,u32,usize,u8,usize,usize,u8,usize,usize,u8,usize"
    //
    // For a named struct fields should look like:
    // Fields: "bat:u32,ts:TestStruct,"
   
    // Now parse out all the field names and their types
    // For tuple structs we automatically make a new name which is the ID
    // of the member
    let mut parsed_fields = Vec::new();
    for (id, field) in fields.split(",").enumerate() {
        // Named structs have a trailing comma, thus we will have one empty
        // string at the end of the CSV list
        if field.len() == 0 { break; }

        let (name, typ) = if is_named_struct {
            let mut spl = field.split(":");
            let name = spl.nth(0).expect("Could not parse member name");
            let typ  = spl.nth(0).expect("Could not parse member type");
            assert!(spl.next() == None, "Unexpected data after member type");
            (name.into(), typ)
        } else {
            (format!("{}", id), field)
        };

        parsed_fields.push((name, typ));
    }

    let mut impltrait = String::new();

    // Start implementation of Safecast for ident
    impltrait += &format!("unsafe impl ::safecast::Safecast for {} {{\n",
                          ident);

    // Implement the `safecast` function
    impltrait += "    fn safecast(&self) {\n";

    // Sum of all the sizes of the individual structures
    impltrait += "        let mut unpadded_struct_size = 0usize;\n";

    for (name, _ty) in parsed_fields {
        // Invoke safecast on this member, this enforces that Safecast is
        // implemented on the type of this member
        impltrait += &format!("        \
            ::safecast::Safecast::safecast(&self.{});\n", name);

        // Accumulate the size of the unpadded structure
        impltrait += &format!("        \
            unpadded_struct_size += ::core::mem::size_of_val(&self.{});\n",
            name);
    }

    // Assert that the size of the entire structure matches the sum of all
    // of it's members. This ensures that there are no padding bytes in the
    // structure.
    //
    // Note: This `size_of::<{}>()` is what prevents us from using a slice
    //       in a structure. This is quite important to have here!
    impltrait += &format!("        \
        assert!(unpadded_struct_size == ::core::mem::size_of::<{}>(),\
            \"Safecast not allowed on structures with padding bytes\");\n",
        ident);

    // Close braces for the `safecast` function and the `impl Safecast`
    impltrait += &format!("    }}\n}}\n");
    impltrait.parse().expect("Failed to convert to TokenStream")
}

