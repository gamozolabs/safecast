# safecast

An attempt to make a procedural macro to support safe casting in Rust.

## Goals

This library is designed to allow for copying raw underlying data between different types in Rust.
This is helpful for handling things like binary files or network protocols. Using this library you
are able to safely create structures and cast/copy between them.

## Safety

This casting/copying is safe given the following:

- The structure is composed only of types which have no invalid/unsafe underlying binary encodings
    - Currently only `u8`, `u16`, `u32`, `u64`, `u128, `usize`, `i8`, `i16`, `i32`, `i64`, `i128, `isize` are considered
      to have these properties.
    - Structures may have structures in them which are also packed and contain only the aforementioned
      types.
    - Fixed sized arrays are also allowed.
    - The current implementation is designed to be extra strict. Things like tuples and such would
      be fine in practice but the goal is to keep things simple for now to make it easier to
      verify.
- The structure is packed such that no padding occurs between fields
    - Since the padding between fields contains undefined values this interface could potentially
      expose them if cast to another type where the padding is readable. Thus we disallow use
      of padding in structures. This doesn't matter much anyways as if you're working with binary
      data it's probably packed anyways.

## Interface

`Safecast::cast_copy_into<T: Safecast + ?Sized>(&self, dest: &mut T)`

This routine allows the casting from an existing structure to another type given the other
type also implemented Safecast. This method is the one used when `T` is `?Sized`, allowing for
us to cast into things like slices/Vecs. This is the core implementation and is used by
`cast()`.

This method will panic unless both self and T are equal in size (in bytes).

`Safecast::cast_copy<T: Safecast>(&self) -> T`

Creates an uninitialized value of type T, and calls `cast_into` on self
to cast it into T. Returns the new value.

This method will panic unless both self and T are equal in size (in bytes).

`Safecast::cast<T: Safecast>(&self) -> &[T]`

Casts `Self` to a slice of `T`s, where `Self` is evenly divisible by `T`.

`Safecast::cast_mut<T: Safecast>(&mut self) -> &mut [T]`

Casts `Self` to a mutable slice of `T`s, where `Self` is evenly divisible by `T`.

## Endianness

I'm not sure if it matches Rust's definition, however I think it is fine for the endianness
to be up to the user to handle. There is no safety violation by having an unexpected
endian swap, thus I'm okay with this not handling endian swaps for you. It is up
to the user to manually swap fields as they use them.

