use crate::registry::sys_runner::{FetchEntity, FetchRes};
use crate::registry::{EntityId, ResourceRegistry};
use std::collections::HashMap;

macro_rules! impl_fetch_res {
    ($($t:ident),+) => {
        impl<$($t: 'static),+> FetchRes for ($($t,)+) {
            fn take(registry: &mut ResourceRegistry) -> Option<Self> {
                Some(($(registry.remove_res::<$t>()?,)+))
            }
            fn put_back(self, registry: &mut ResourceRegistry) {
                #[allow(non_snake_case)]
                let ($($t,)+) = self;
                $(registry.insert_res($t);)+
            }
        }
    };
}

impl_fetch_res!(A);
impl_fetch_res!(A, B);
impl_fetch_res!(A, B, C);
impl_fetch_res!(A, B, C, D);
impl_fetch_res!(A, B, C, D, E);

macro_rules! impl_fetch_entity {
    ($($t:ident),+) => {
        impl<$($t: 'static),+> FetchEntity for ($($t,)+) {
            type Maps = ($(HashMap<EntityId, $t>,)+);

            fn take(registry: &mut ResourceRegistry) -> Option<Self::Maps> {
                Some(($(registry.remove_map::<$t>()?,)+))
            }

            fn put_back(maps: Self::Maps, registry: &mut ResourceRegistry) {
                #[allow(non_snake_case)]
                let ($($t,)+) = maps;
                $(registry.insert_map($t);)+
            }
        }
    };
}

impl_fetch_entity!(A);
impl_fetch_entity!(A, B);
impl_fetch_entity!(A, B, C);
impl_fetch_entity!(A, B, C, D);
impl_fetch_entity!(A, B, C, D, E);
