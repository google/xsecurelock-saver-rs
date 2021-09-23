pub mod diagnostic;
pub mod renderer;
mod wgpu_render_pass;
mod wgpu_renderer;
mod wgpu_resources;
mod wgpu_type_converter;

use bevy_window::{WindowDescriptor, WindowId};
pub use wgpu_render_pass::*;
pub use wgpu_renderer::*;
pub use wgpu_resources::*;

use bevy_app::prelude::*;
use bevy_ecs::{
    system::{IntoExclusiveSystem, IntoSystem},
    world::World,
};
use bevy_render::{
    renderer::{shared_buffers_update_system, RenderResourceContext, SharedBuffers},
    RenderStage,
};
use futures_lite::future;
use raw_window_handle::{unix::XlibHandle, HasRawWindowHandle, RawWindowHandle};
use renderer::WgpuRenderResourceContext;
use std::{borrow::Cow, env, os::unix::prelude::OsStringExt};

#[derive(Clone, Copy)]
pub enum WgpuFeature {
    DepthClamping,
    TextureCompressionBc,
    TimestampQuery,
    PipelineStatisticsQuery,
    MappablePrimaryBuffers,
    SampledTextureBindingArray,
    SampledTextureArrayDynamicIndexing,
    SampledTextureArrayNonUniformIndexing,
    UnsizedBindingArray,
    MultiDrawIndirect,
    MultiDrawIndirectCount,
    PushConstants,
    AddressModeClampToBorder,
    NonFillPolygonMode,
    TextureCompressionEtc2,
    TextureCompressionAstcLdr,
    TextureAdapterSpecificFormatFeatures,
    ShaderFloat64,
    VertexAttribute64Bit,
}

#[derive(Default, Clone)]
pub struct WgpuFeatures {
    pub features: Vec<WgpuFeature>,
}

#[derive(Debug, Clone)]
pub struct WgpuLimits {
    pub max_bind_groups: u32,
    pub max_dynamic_uniform_buffers_per_pipeline_layout: u32,
    pub max_dynamic_storage_buffers_per_pipeline_layout: u32,
    pub max_sampled_textures_per_shader_stage: u32,
    pub max_samplers_per_shader_stage: u32,
    pub max_storage_buffers_per_shader_stage: u32,
    pub max_storage_textures_per_shader_stage: u32,
    pub max_uniform_buffers_per_shader_stage: u32,
    pub max_uniform_buffer_binding_size: u32,
    pub max_push_constant_size: u32,
}

impl Default for WgpuLimits {
    fn default() -> Self {
        let default = wgpu::Limits::default();
        WgpuLimits {
            max_bind_groups: default.max_bind_groups,
            max_dynamic_uniform_buffers_per_pipeline_layout: default
                .max_dynamic_uniform_buffers_per_pipeline_layout,
            max_dynamic_storage_buffers_per_pipeline_layout: default
                .max_dynamic_storage_buffers_per_pipeline_layout,
            max_sampled_textures_per_shader_stage: default.max_sampled_textures_per_shader_stage,
            max_samplers_per_shader_stage: default.max_samplers_per_shader_stage,
            max_storage_buffers_per_shader_stage: default.max_storage_buffers_per_shader_stage,
            max_storage_textures_per_shader_stage: default.max_storage_textures_per_shader_stage,
            max_uniform_buffers_per_shader_stage: default.max_uniform_buffers_per_shader_stage,
            max_uniform_buffer_binding_size: default.max_uniform_buffer_binding_size,
            max_push_constant_size: default.max_push_constant_size,
        }
    }
}

#[derive(Default)]
pub struct WgpuPlugin;

impl Plugin for WgpuPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let render_system = get_wgpu_render_system(app.world_mut());
        app.add_system_to_stage(RenderStage::Render, render_system.exclusive_system())
            .add_system_to_stage(
                RenderStage::PostRender,
                shared_buffers_update_system.system(),
            );
    }
}

pub fn get_wgpu_render_system(world: &mut World) -> impl FnMut(&mut World) {
    let options = world
        .get_resource::<WgpuOptions>()
        .cloned()
        .unwrap_or_else(WgpuOptions::default);
    let mut wgpu_renderer = future::block_on(WgpuRenderer::new(options));

    let resource_context = WgpuRenderResourceContext::new(wgpu_renderer.device.clone());
    world.insert_resource::<Box<dyn RenderResourceContext>>(Box::new(resource_context));
    world.insert_resource(SharedBuffers::new(4096));
    move |world| {
        wgpu_renderer.update(world);
    }
}

#[derive(Default, Clone)]
pub struct WgpuOptions {
    pub device_label: Option<Cow<'static, str>>,
    pub backend: WgpuBackend,
    pub power_pref: WgpuPowerOptions,
    pub features: WgpuFeatures,
    pub limits: WgpuLimits,
}

#[derive(Clone)]
pub enum WgpuBackend {
    Auto,
    Vulkan,
    Metal,
    Dx12,
    Dx11,
    Gl,
    BrowserWgpu,
}

impl WgpuBackend {
    fn from_env() -> Self {
        if let Ok(backend) = std::env::var("BEVY_WGPU_BACKEND") {
            match backend.to_lowercase().as_str() {
                "vulkan" => WgpuBackend::Vulkan,
                "metal" => WgpuBackend::Metal,
                "dx12" => WgpuBackend::Dx12,
                "dx11" => WgpuBackend::Dx11,
                "gl" => WgpuBackend::Gl,
                "webgpu" => WgpuBackend::BrowserWgpu,
                other => panic!("Unknown backend: {}", other),
            }
        } else {
            WgpuBackend::Auto
        }
    }
}

impl Default for WgpuBackend {
    fn default() -> Self {
        Self::from_env()
    }
}

#[derive(Clone)]
pub enum WgpuPowerOptions {
    HighPerformance,
    Adaptive,
    LowPower,
}

impl Default for WgpuPowerOptions {
    fn default() -> Self {
        WgpuPowerOptions::HighPerformance
    }
}

/// External X window.
pub struct ExternalXWindow {
    display: *mut x11::xlib::Display,
    handle: x11::xlib::Window,
    pub window_id: WindowId,
}

unsafe impl Send for ExternalXWindow {}
unsafe impl Sync for ExternalXWindow {}

impl ExternalXWindow {
    /// Open a connection to the X Display attached to the given window.
    pub fn new(handle: x11::xlib::Window) -> Self {
        let display = env::var_os("DISPLAY").expect("No X11 $DISPLAY set");
        let display =
            std::ffi::CString::new(display.into_vec()).expect("$DISPLAY was not a valid CString");
        let display = unsafe { x11::xlib::XOpenDisplay(display.as_ptr()) };
        if display.is_null() {
            panic!("Failed to open display");
        }
        Self {
            display,
            handle,
            window_id: WindowId::primary(),
        }
    }

    pub fn bevy_window_descriptor(&self) -> WindowDescriptor {
        let mut attributes = unsafe { std::mem::zeroed::<x11::xlib::XWindowAttributes>() };
        if unsafe { x11::xlib::XGetWindowAttributes(self.display, self.handle, &mut attributes) }
            == 0
        {
            panic!("Failed to get window attributes");
        }

        WindowDescriptor {
            width: attributes.width as f32,
            height: attributes.height as f32,
            resizable: false,
            ..Default::default()
        }
    }
}

impl Drop for ExternalXWindow {
    fn drop(&mut self) {
        unsafe { x11::xlib::XCloseDisplay(self.display) };
        self.display = std::ptr::null_mut();
    }
}

unsafe impl HasRawWindowHandle for ExternalXWindow {
    fn raw_window_handle(&self) -> RawWindowHandle {
        RawWindowHandle::Xlib(XlibHandle {
            window: self.handle,
            display: self.display.cast(),
            ..XlibHandle::empty()
        })
    }
}
