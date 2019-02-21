#[cfg(test)]
mod tests {
    use safecast::Safecast;
        
    #[derive(Safecast, Debug, Clone, Copy, PartialEq)]
    #[repr(C)]
    struct Au32(u32);
    
    #[derive(Safecast, Debug, Clone, Copy, PartialEq)]
    #[repr(C)]
    struct Au32Pad(u32, u8);


    #[test]
    fn check_cast_copy() {
        assert!([0x41u8; 4].cast_copy::<Au32>() == Au32(0x41414141));
    }
    
    #[test]
    fn check_cast_sized() {
        let sized = vec![0x20u8; 4];
        assert!(sized[0..4].cast_copy::<Au32>() == Au32(0x20202020));
    }
    
    #[test]
    fn check_cast_into_sized() {
        let val = Au32(0x90909090);
        let mut output = vec![0u8; 4];
        val.cast_copy_into(&mut output[..]);
        assert!(output == [0x90; 4]);
    }
    
    #[test]
    fn check_cast() {
        assert!([0x41u8; 4].cast::<Au32>() == &[Au32(0x41414141)]);
    }
    
    #[test]
    fn check_cast_multiple() {
        assert!([0x41u8; 8].cast::<Au32>() == &[Au32(0x41414141); 2]);
    }
    
    #[test]
    #[should_panic="Cast alignment mismatch"]
    fn check_cast_align() {
        assert!([0x41u8; 6][2..6].cast::<Au32>() == &[Au32(0x41414141); 2]);
    }
    
    #[test]
    #[should_panic="cast src cannot be evenly divided by T"]
    fn check_cast_mismatch() {
        assert!([0x41u8; 3].cast::<Au32>() == &[Au32(0x41414141); 2]);
    }
    
    #[test]
    #[should_panic="Safecast not allowed on structures with padding bytes"]
    fn check_cast_padding() {
        assert!([0x41u8; 8].cast_copy::<Au32Pad>() == Au32Pad(0x41414141, 0x41));
    }
    
    #[test]
    #[should_panic="Safecast not allowed on structures with padding bytes"]
    fn check_cast_padding_into_sized() {
        let val = Au32Pad(0x90909090, 0x90);
        let mut output = vec![0u8; 8];
        val.cast_copy_into(&mut output[..]);
        assert!(output == [0x90; 8]);
    }
}

