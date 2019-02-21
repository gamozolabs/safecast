#![no_std]

/// Re-export the Safecast derive procedural macro
pub use bytesafe::Safecast;

pub unsafe trait Safecast {
    /// Function that does runtime checks on the underlying structure to
    /// validate things that we could not check at compile time (like checking
    /// for padding bytes). This must be called prior to ever using the
    /// `Safecast` structures as POD.
    fn safecast(&self);

    /// Copy the underlying bytes of `self` into a different type `T` given
    /// they're both representing plain-old-data with no padding and they have
    /// identical sizes.
    fn cast_copy_into<T: Safecast + ?Sized>(&self, dest: &mut T) {
        // Make sure we're not working with zero-size-types
        assert!(core::mem::size_of_val(self) > 0, "ZST not allowed");
        assert!(core::mem::size_of_val(dest) > 0, "ZST not allowed");

        // Make sure sizes match between the two things
        assert!(core::mem::size_of_val(self) == core::mem::size_of_val(dest)
                "Size mismatch in cast_copy_into");

        // Validate runtime checks on the structures we're working with
        Safecast::safecast(self);
        Safecast::safecast(dest);

        // At this point the structures have been validated to have no padding
        // bytes, and both are entirely composed of types that also have no
        // padding and are only PoD

        // Perform the copy
        unsafe {
            core::ptr::copy_nonoverlapping(
                self as *const Self as *const u8,
                dest as *mut   T    as *mut   u8,
                core::mem::size_of_val(self));
        }
    }

    /// Create a new value of type `T`, copy the raw byte contents of `self`
    /// into it, and return it.
    fn cast_copy<T: Safecast>(&self) -> T {
        // Safe to use uninitialized here because we will fill in _all_ the
        // output bytes
        let mut ret: T = unsafe { core::mem::uninitialized() };
        self.cast_copy_into(&mut ret);
        ret
    }

    /// Cast `self` into a slice of type `T`s
    ///
    /// Since casting is only safe if alignment matches, this can panic if
    /// the types do not have the same alignments
    fn cast<T: Safecast>(&self) -> &[T] {
        // Make sure we're not working with zero-size-types
        assert!(core::mem::size_of_val(self) > 0, "ZST not allowed");
        assert!(core::mem::size_of::<T>()    > 0, "ZST not allowed");
        
        // Validate runtime checks on the input (we can't work on the output
        // yet)
        Safecast::safecast(self);
        
        // Validate alignment
        let src_ptr = self as *const Self as *const u8 as usize;
        assert!(core::mem::align_of::<T>() > 0 &&
                (src_ptr % core::mem::align_of::<T>()) == 0,
                "Cast alignment mismatch");

        // Validate that self is evenly divisible by T
        let dest_sz = core::mem::size_of::<T>();
        let src_sz  = core::mem::size_of_val(self);
        assert!((src_sz % dest_sz) == 0,
            "cast src cannot be evenly divided by T");

        // Perform the cast!
        let casted = unsafe {
            core::slice::from_raw_parts(self as *const Self as *const T,
                                        src_sz / dest_sz)
        };

        // Validate runtime checks on output
        Safecast::safecast(casted);

        casted
    }

    /// Cast `self` into a mutable slice of type `T`s
    ///
    /// Since casting is only safe if alignment matches, this can panic if
    /// the types do not have the same alignments
    fn cast_mut<T: Safecast>(&mut self) -> &mut [T] {
        // Make sure we're not working with zero-size-types
        assert!(core::mem::size_of_val(self) > 0, "ZST not allowed");
        assert!(core::mem::size_of::<T>()    > 0, "ZST not allowed");
        
        // Validate runtime checks on the input (we can't work on the output
        // yet)
        Safecast::safecast(self);
        
        // Validate alignment
        let src_ptr = self as *const Self as *const u8 as usize;
        assert!(core::mem::align_of::<T>() > 0 &&
                (src_ptr % core::mem::align_of::<T>()) == 0,
                "Cast alignment mismatch");

        // Validate that self is evenly divisible by T
        let dest_sz = core::mem::size_of::<T>();
        let src_sz  = core::mem::size_of_val(self);
        assert!((src_sz % dest_sz) == 0,
            "cast src cannot be evenly divided by T");

        // Perform the cast!
        let casted = unsafe {
            core::slice::from_raw_parts_mut(self as *mut Self as *mut T,
                                            src_sz / dest_sz)
        };

        // Validate runtime checks on output
        Safecast::safecast(casted);

        casted
    }
}

// Create impls for the root types we can build upon
// The safecast() function implementation is responsible for checking that
// there is no padding bytes in the structures. Since these types are just
// primitives, the safecast() routine just does nothing at all

unsafe impl Safecast for u8    { fn safecast(&self) {} }
unsafe impl Safecast for u16   { fn safecast(&self) {} }
unsafe impl Safecast for u32   { fn safecast(&self) {} }
unsafe impl Safecast for u64   { fn safecast(&self) {} }
unsafe impl Safecast for u128  { fn safecast(&self) {} }
unsafe impl Safecast for usize { fn safecast(&self) {} }
unsafe impl Safecast for i8    { fn safecast(&self) {} }
unsafe impl Safecast for i16   { fn safecast(&self) {} }
unsafe impl Safecast for i32   { fn safecast(&self) {} }
unsafe impl Safecast for i64   { fn safecast(&self) {} }
unsafe impl Safecast for i128  { fn safecast(&self) {} }
unsafe impl Safecast for isize { fn safecast(&self) {} }

// We implement `Safecast` for slices which also are composed of only
// `Safecast` members. We cannot put a slice in a structure that derives
// `Safecast` as we do a `size_of::<T>()` and this requires that the structure
// has a fixed size at compile time.
// If you put a [T] in a structure you try to derive `Safecast` on you will
// get a compile-time error due to `Sized` not being implemented when the
// `size_of` check occurs
//
// This impl allows us to use dynamic-sized slices as sources and destinations
// for our cast routines, however the types themselves cannot ever be dynamic
//
// I think this is safe as we cannot construct anything that contains a [T]
// as it's ?Sized. We cannot make a structure member that's ?Sized, and
// we cannot make an array/vector/slice out of members which are ?Sized so I
// don't see any way this can be used to violate safety.
//
// We invoke the safecast function on one member of the slice to ensure that
// runtime checks are done on T to validate safety
unsafe impl<T: Safecast> Safecast for [T] { fn safecast(&self) { Safecast::safecast(&self[0]) }}

// Generic fixed-sized array impls
// We invoke the safecast function on one member of the array to ensure that
// runtime checks are done on T to validate safety

unsafe impl<T: Safecast> Safecast for [T;   1] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;   2] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;   3] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;   4] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;   5] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;   6] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;   7] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;   8] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;   9] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  10] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  11] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  12] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  13] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  14] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  15] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  16] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  17] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  18] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  19] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  20] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  21] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  22] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  23] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  24] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  25] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  26] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  27] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  28] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  29] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  30] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  31] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  32] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  33] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  34] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  35] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  36] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  37] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  38] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  39] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  40] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  41] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  42] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  43] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  44] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  45] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  46] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  47] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  48] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  49] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  50] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  51] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  52] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  53] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  54] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  55] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  56] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  57] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  58] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  59] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  60] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  61] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  62] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  63] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  64] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  65] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  66] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  67] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  68] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  69] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  70] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  71] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  72] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  73] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  74] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  75] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  76] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  77] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  78] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  79] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  80] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  81] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  82] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  83] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  84] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  85] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  86] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  87] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  88] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  89] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  90] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  91] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  92] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  93] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  94] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  95] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  96] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  97] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  98] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T;  99] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 100] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 101] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 102] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 103] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 104] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 105] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 106] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 107] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 108] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 109] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 110] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 111] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 112] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 113] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 114] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 115] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 116] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 117] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 118] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 119] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 120] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 121] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 122] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 123] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 124] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 125] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 126] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 127] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 128] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 129] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 130] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 131] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 132] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 133] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 134] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 135] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 136] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 137] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 138] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 139] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 140] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 141] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 142] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 143] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 144] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 145] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 146] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 147] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 148] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 149] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 150] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 151] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 152] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 153] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 154] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 155] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 156] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 157] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 158] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 159] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 160] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 161] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 162] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 163] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 164] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 165] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 166] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 167] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 168] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 169] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 170] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 171] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 172] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 173] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 174] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 175] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 176] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 177] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 178] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 179] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 180] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 181] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 182] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 183] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 184] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 185] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 186] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 187] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 188] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 189] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 190] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 191] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 192] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 193] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 194] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 195] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 196] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 197] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 198] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 199] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 200] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 201] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 202] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 203] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 204] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 205] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 206] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 207] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 208] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 209] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 210] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 211] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 212] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 213] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 214] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 215] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 216] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 217] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 218] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 219] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 220] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 221] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 222] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 223] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 224] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 225] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 226] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 227] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 228] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 229] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 230] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 231] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 232] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 233] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 234] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 235] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 236] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 237] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 238] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 239] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 240] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 241] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 242] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 243] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 244] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 245] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 246] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 247] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 248] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 249] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 250] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 251] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 252] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 253] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 254] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 255] { fn safecast(&self) { Safecast::safecast(&self[0]) }}
unsafe impl<T: Safecast> Safecast for [T; 256] { fn safecast(&self) { Safecast::safecast(&self[0]) }}

