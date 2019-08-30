use crate::space::*;
use crate::AtomURIDCache;
use core::UriBound;
use std::os::raw::*;
use urid::{URIDBound, URID};

pub trait ScalarAtom: URIDBound {
    type InternalType: Copy + Sized + 'static;

    fn space_as_body(space: Space<Self>) -> Option<Self::InternalType> {
        unsafe { space.split_type::<Self::InternalType>() }.map(|(value, _)| *value)
    }

    fn write_body<'a>(
        space: &mut dyn MutSpace<'a>,
        value: Self::InternalType,
        urids: &Self::CacheType,
    ) -> Option<&'a mut Self::InternalType> {
        unsafe {
            space
                .create_atom_frame::<Self>(urids)
                .and_then(|mut frame| (&mut frame as &mut dyn MutSpace).write(&value, true))
        }
    }
}

macro_rules! make_scalar_atom {
    ($atom:ty, $internal:ty, $uri:expr, $urid:expr) => {
        unsafe impl UriBound for $atom {
            const URI: &'static [u8] = $uri;
        }

        impl URIDBound for $atom {
            type CacheType = AtomURIDCache;

            fn urid(cache: &AtomURIDCache) -> URID<$atom> {
                #[allow(clippy::redundant_closure_call)]
                ($urid)(cache)
            }
        }

        impl ScalarAtom for $atom {
            type InternalType = $internal;
        }
    };
}

pub struct Double;

make_scalar_atom!(
    Double,
    c_double,
    sys::LV2_ATOM__Double,
    |urids: &AtomURIDCache| urids.double
);

pub struct Float;

make_scalar_atom!(
    Float,
    c_float,
    sys::LV2_ATOM__Float,
    |urids: &AtomURIDCache| urids.float
);

pub struct Int;

make_scalar_atom!(Int, c_int, sys::LV2_ATOM__Int, |urids: &AtomURIDCache| {
    urids.int
});

pub struct Long;

make_scalar_atom!(
    Long,
    c_long,
    sys::LV2_ATOM__Long,
    |urids: &AtomURIDCache| urids.long
);

pub struct Bool;

make_scalar_atom!(
    Bool,
    c_int,
    sys::LV2_ATOM__Bool,
    |urids: &AtomURIDCache| urids.bool
);

pub struct AtomURID;

make_scalar_atom!(
    AtomURID,
    URID,
    sys::LV2_ATOM__URID,
    |urids: &AtomURIDCache| urids.urid
);

#[cfg(test)]
mod tests {
    use crate::scalar::*;
    use std::mem::size_of;
    use sys::*;
    use urid::URIDCache;

    #[test]
    fn test_scalar_retrieval() {
        let mut map_interface = urid::mapper::URIDMap::new().make_map_interface();
        let map = map_interface.map();
        let urids = crate::AtomURIDCache::from_map(&map).unwrap();

        macro_rules! test_atom {
            ($orig:ident, $atom:ty, $value:expr) => {
                let original_atom = $orig {
                    atom: sys::LV2_Atom {
                        type_: <$atom>::urid(&urids).get(),
                        size: size_of::<<$atom as ScalarAtom>::InternalType>() as u32,
                    },
                    body: $value.into(),
                };

                let space: Space<$atom> =
                    unsafe { Space::from_atom(&original_atom.atom, &urids) }.unwrap();
                let value = <$atom>::space_as_body(space).unwrap();
                assert_eq!($value, value);
            };
        }

        test_atom!(LV2_Atom_Double, Double, 42.0);
        test_atom!(LV2_Atom_Float, Float, 42.0);
        test_atom!(LV2_Atom_Long, Long, 42);
        test_atom!(LV2_Atom_Int, Int, 42);
        test_atom!(LV2_Atom_Bool, Bool, 1);
        test_atom!(LV2_Atom_URID, AtomURID, urids.urid.get());
    }

    #[test]
    fn test_scalar_writing() {
        let mut map_interface = urid::mapper::URIDMap::new().make_map_interface();
        let urids = crate::AtomURIDCache::from_map(&map_interface.map()).unwrap();

        let mut memory: [u64; 256] = [0; 256];
        let raw_memory: &mut [u8] = unsafe {
            std::slice::from_raw_parts_mut(memory.as_mut_ptr() as *mut u8, 256 * size_of::<u64>())
        };

        macro_rules! test_atom {
            ($orig:ident, $atom:ty, $value:expr) => {
                let mut space = RootMutSpace::new(raw_memory);
                <$atom>::write_body(&mut space, $value, &urids).unwrap();
                let raw_atom = unsafe { &*(raw_memory.as_ptr() as *const $orig) };
                assert_eq!(
                    raw_atom.atom.size as usize,
                    size_of::<<$atom as ScalarAtom>::InternalType>()
                );
                assert_eq!(raw_atom.atom.type_, <$atom>::urid(&urids).get());
                assert_eq!(raw_atom.body, $value);
            };
        }

        test_atom!(LV2_Atom_Double, Double, 42.0);
        test_atom!(LV2_Atom_Float, Float, 42.0);
        test_atom!(LV2_Atom_Long, Long, 42);
        test_atom!(LV2_Atom_Int, Int, 42);
        test_atom!(LV2_Atom_Bool, Bool, 1);
        test_atom!(LV2_Atom_URID, AtomURID, urids.urid.into_general());
    }
}
