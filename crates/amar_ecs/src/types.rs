use std::{
    alloc::{Layout, alloc, dealloc},
    any::TypeId,
    collections::HashSet,
    ptr::NonNull,
};

use getset::CopyGetters;

/// we can use Vec<TypeId> as index : \[Position] != \[Position,Health]
/// order matters in comparaison, so to eliminate order difference, we must keep the vec ordered by TypeId
pub type ArchetypeComponents = Vec<TypeId>;

#[derive(Debug, Clone, Copy, CopyGetters)]
pub struct ComponentInfo {
    #[getset(get_copy = "pub")]
    type_id: TypeId,
    #[getset(get_copy = "pub")]
    size: u32,
    /// smallest power of 2 bigger then self.size  
    /// used for column allocation
    #[getset(get_copy = "pub")]
    align: u32,
    drop_fn: unsafe fn(*mut u8),
}
impl ComponentInfo {
    pub fn drop(&self, ptr: NonNull<u8>) {
        unsafe {
            (self.drop_fn)(ptr.as_ptr());
        }
    }

    pub fn new<Comp: Component>() -> Self {
        Self {
            type_id: TypeId::of::<Comp>(),
            size: size_of::<Comp>() as u32,
            align: align_of::<Comp>() as u32,
            drop_fn: Self::drop_component::<Comp>,
        }
    }
    unsafe fn drop_component<T>(ptr: *mut u8) {
        unsafe { std::ptr::drop_in_place(ptr.cast::<T>()) };
    }
}

#[derive(Eq, Hash, PartialEq, Clone, Copy, Debug, Default)]
pub struct ArchetypeId(u32);

impl ArchetypeId {
    pub fn next(&mut self) -> Self {
        let old = *self;
        self.0 += 1;
        return old;
    }
}

#[derive(Eq, Hash, PartialEq, Clone, Debug, Copy, Default)]
pub struct EntityId(u32);

impl EntityId {
    pub fn next(&mut self) -> Self {
        let old = *self;
        self.0 += 1;
        return old;
    }
}

pub type ArchetypeSet = HashSet<ArchetypeId>;

pub trait Resource: 'static {}
pub trait Component: 'static {}

#[derive(Debug, Clone)]
pub struct EntityInfo {
    pub archetype_id: ArchetypeId,
    pub row: u32,
}
pub struct ArchetypeRecord {
    pub column: u32,
}

pub struct RawComponent {
    buffer: NonNull<u8>,
    pub info: ComponentInfo,
}

impl RawComponent {
    /// Copies the content of `ptr` into a new allocation, handling ZSTs gracefully.
    pub fn new_from_ptr(ptr: NonNull<u8>, info: ComponentInfo) -> Self {
        let size = info.size();

        // Pass through the existing dangling/aligned pointer for zero-sized types
        if size == 0 {
            return Self { buffer: ptr, info };
        }

        let align = info.align();
        let layout = Layout::from_size_align(size as usize, align as usize)
            .expect("Invalid layout parameters in ComponentInfo");

        let buffer = unsafe {
            let dst_raw = alloc(layout);
            let dst_non_null =
                NonNull::new(dst_raw).expect("Memory allocation failed for RawComponent");

            std::ptr::copy_nonoverlapping(ptr.as_ptr(), dst_non_null.as_ptr(), size as usize);
            dst_non_null
        };

        Self { buffer, info }
    }
    /// Copies the content of `ptr` into a new allocation, handling ZSTs gracefully.
    pub fn new<Comp: Component>(component: Comp) -> Self {
        let info = ComponentInfo::new::<Comp>();
        let component_ptr = NonNull::from(&component).cast();
        let raw_component = Self::new_from_ptr(component_ptr, info);
        std::mem::forget(component);
        raw_component
    }
    fn free_buffer(&self) {
        let size = self.info.size() as usize;
        if size > 0 {
            let layout = Layout::from_size_align(size, self.info.align() as usize)
                .expect("Invalid layout parameters in RawComponent");
            unsafe {
                dealloc(self.buffer.as_ptr(), layout);
            }
        }
    }
    pub fn move_to(self, dst: NonNull<u8>) {
        let size = self.info.size() as usize;
        if size != 0 {
            unsafe {
                std::ptr::copy_nonoverlapping(self.buffer.as_ptr(), dst.as_ptr(), size);
            }
        }

        // Suppress `RawComponent::drop` so `self.info.drop()` is NOT called on copied data
        self.free_buffer();
        std::mem::forget(self);
    }

    /// Casts and moves the raw buffer into an owned instance of `Comp`.
    /// Panics if `Comp` does not match the internal `TypeId`.
    pub fn into_cast<Comp: Component>(self) -> Comp {
        assert_eq!(
            self.info.type_id(),
            TypeId::of::<Comp>(),
            "Type mismatch during RawComponent::into_cast"
        );

        unsafe {
            // this creates a copy in the stack
            // so we must free the old pointer
            let component = std::ptr::read(self.buffer.as_ptr().cast::<Comp>());

            // 3. Suppress `RawComponent::drop` to prevent running self.info.drop() or double dealloc
            self.free_buffer();
            std::mem::forget(self);

            component
        }
    }
}

impl Drop for RawComponent {
    fn drop(&mut self) {
        // 1. Run the inner component's destructor (even for ZSTs)
        self.info.drop(self.buffer);

        self.free_buffer();
    }
}
