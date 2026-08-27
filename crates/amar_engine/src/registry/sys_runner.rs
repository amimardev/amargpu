use wgpu::{SurfaceTexture, TextureView};

use crate::{
    keys,
    registry::{ResourceRegistry, registry_view::PluginRegistryView},
    state::GlobalState,
    texture::Texture,
};

pub trait FetchRes: Sized {
    fn take(registry: &mut ResourceRegistry) -> Option<Self>;
    fn put_back(self, registry: &mut ResourceRegistry);
}
pub trait FetchEntity: Sized {
    type Maps;
    fn take(registry: &mut ResourceRegistry) -> Option<Self::Maps>;
    fn put_back(maps: Self::Maps, registry: &mut ResourceRegistry);
}

pub struct SysRunner<'a> {
    registry: &'a mut ResourceRegistry,
}
impl<'a> SysRunner<'a> {
    pub fn glb(&self) -> &GlobalState {
        self.registry.get_res::<GlobalState>().unwrap()
    }
    pub fn glb_mut(&mut self) -> &mut GlobalState {
        self.registry.get_res_mut::<GlobalState>().unwrap()
    }
    pub fn new(registry: &'a mut ResourceRegistry) -> Self {
        Self { registry }
    }
    pub fn exec_maps<Tup: FetchEntity, R>(
        &mut self,
        f: impl FnOnce(&mut Tup::Maps, PluginRegistryView) -> R,
    ) -> Option<R> {
        let mut maps = Tup::take(self.registry)?;
        let result = f(&mut maps, self.registry.view());
        Tup::put_back(maps, self.registry);
        Some(result)
    }

    pub fn exec<TupE: FetchEntity, TupR: FetchRes, R>(
        &mut self,
        sys: impl FnOnce(&mut TupE::Maps, &mut TupR, PluginRegistryView) -> R,
    ) -> Option<R> {
        let mut maps = TupE::take(self.registry)?;
        let mut resources = TupR::take(self.registry)?;

        let result = sys(&mut maps, &mut resources, self.registry.view());

        TupE::put_back(maps, self.registry);
        TupR::put_back(resources, self.registry);

        Some(result)
    }

    pub fn exec_res<Tup: FetchRes, R>(
        &mut self,
        f: impl FnOnce(&mut Tup, PluginRegistryView) -> R,
    ) -> Option<R> {
        let mut tup = Tup::take(self.registry)?;
        let result = f(&mut tup, self.registry.view());
        tup.put_back(self.registry);
        Some(result)
    }
}

pub struct RenderSysRunner<'a> {
    pub(super) registry: &'a mut ResourceRegistry,
    surface_texture: &'a SurfaceTexture,
}

impl<'a> RenderSysRunner<'a> {
    pub fn new(registry: &'a mut ResourceRegistry, surface_texture: &'a SurfaceTexture) -> Self {
        Self {
            registry,
            surface_texture,
        }
    }
    pub fn glb(&self) -> &GlobalState {
        self.registry.get_res::<GlobalState>().unwrap()
    }

    pub fn exec_render<TupR: FetchRes, R>(
        &mut self,
        render_pass: &mut wgpu::RenderPass,
        sys: impl FnOnce(&TupR, &mut wgpu::RenderPass, PluginRegistryView) -> R,
    ) -> Option<R> {
        let mut resources = TupR::take(self.registry)?;

        let result = sys(&mut resources, render_pass, self.registry.view());

        TupR::put_back(resources, self.registry);

        Some(result)
    }
    pub fn get_depth_texture(&self) -> &Texture {
        self.registry
            .get_by_label::<Texture>(keys::texture::DEPTH)
            .first()
            .unwrap()
    }
    pub fn get_surface_texture_view(&self) -> TextureView {
        self.surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default())
    }
}
