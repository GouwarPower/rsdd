use std::ptr;

#[macro_export]
macro_rules! BITFIELD {
    ($base:ident $field:ident: $fieldtype:ty [
        $($thing:ident $set_thing:ident[$r:expr],)+
    ]) => {
        impl $base {$(
            #[inline]
            pub fn $thing(&self) -> $fieldtype {
                let size = mem::size_of::<$fieldtype>() * 8;
                self.$field << (size - $r.end) >> (size - $r.end + $r.start)
            }
            #[inline]
            pub fn $set_thing(&mut self, val: $fieldtype) {
                let mask = ((1 << ($r.end - $r.start)) - 1) << $r.start;
                self.$field &= !mask;
                self.$field |= (val << $r.start) & mask;
            }
        )+}
    }
}


/// custom allocations for zeroed vectors
pub fn zero_vec<T>(sz: usize) -> Vec<T> {
    let mut v : Vec<T> = Vec::with_capacity(sz);
    unsafe {
        let vec_ptr = v.as_mut_ptr();
        ptr::write_bytes(vec_ptr, 0, sz as usize);
        v.set_len(sz);
    }
    return v
}