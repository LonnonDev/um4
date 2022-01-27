use std::sync::Arc;

use vulkano::{instance::Instance, Version, device::{physical::PhysicalDevice, DeviceExtensions, Device, Features}, swapchain::Swapchain, pipeline::graphics::viewport::Viewport, image::{SwapchainImage, ImageAccess, view::ImageView}, render_pass::{RenderPass, Framebuffer}, sync::{self, GpuFuture}};
use vulkano_win::VkSurfaceBuild;
use winit::{event_loop::EventLoop, window::{WindowBuilder, Window}, platform::unix::EventLoopExtUnix};


#[test]
fn triangle() {
    // Black box that has Vulkan Stuff written on it
    let instance = {
        let extensions = vulkano_win::required_extensions();
        Instance::new(None, Version::V1_2, &extensions, None).unwrap()
    };

    // Get's the first gpu
    let physical = PhysicalDevice::enumerate(&instance).next().unwrap();

    // Create the event loop
    let event_loop: EventLoop<()> = EventLoop::new_any_thread();

    // What vulkan can render to, where vulkan can stick the output of stuff
    let surface = WindowBuilder::new().build_vk_surface(&event_loop, instance.clone()).unwrap();

    // QueueFamily is the type of operation we can do and Queue is the instance of the type of operation
    // Makes sure queue supports graphics and surface is alsos supported
    let queue_family = physical.queue_families().find(|&q| {
        q.supports_graphics() && surface.is_supported(q).unwrap_or(false)
    }).unwrap();

    // Features
    let mut physical_features = Features::none();

    // Software repsentation of hardware being stored in physical
    let device_ext = DeviceExtensions { khr_swapchain: true, .. DeviceExtensions::none() };
    // The device and priority
    let (device, mut queues) = Device::new(physical, &physical_features, &device_ext,
[(queue_family, 0.5)].iter().cloned()).unwrap();

    // A single queue
    let queue = queues.next().unwrap();

    // How we show the user what frame we rendered
    let (mut swapchain, images) = {
        // Gets capabilities of the surface's physical device
        let caps = surface.capabilities(physical).unwrap();
        // supported usage of the image
        let usage = caps.supported_usage_flags;
        let alpha = caps.supported_composite_alpha.iter().next().unwrap();
        // Supported formats
        let format = caps.supported_formats[0].0;
        // Size of window
        let dimensions: [u32; 2] = surface.window().inner_size().into();
    
        // starts the swapchain
        Swapchain::start(device.clone(), surface.clone())
            .num_images(caps.min_image_count)
            .format(format)
            .dimensions(dimensions)
            .usage(usage)
            .sharing_mode(&queue)
            .composite_alpha(alpha)
            .build()
            .unwrap()
    };

    // Where the inputs and outputs for the graphics hardware is
    let render_pass = vulkano::single_pass_renderpass!(
        device.clone(),
        attachments: {
            // Frame buffer for color
            color: {
                // should be cleared at the start of a render pass
                load: Clear,
                // store the result of the render pass
                store: Store,
                // format of the render attachment
                format: swapchain.format(),
                // how many samples the render attachment should have
                samples: 1,
            }
        },
        // Passes the color buffer
        pass: {
            color: [color],
            depth_stencil: {}
        }
    )
    .unwrap();

    // window viewport
    let mut viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: [0.0, 0.0],
        depth_range: 0.0..1.0,
    };

    // Framebuffer
    let mut framebuffers = window_size_dependent_setup(&images, render_pass.clone(), &mut viewport);

    // variable to say when to recreate the swapchain
    let mut recreate_swapchain = false;

    // 
    let mut previous_frame_end = Some(Box::new(sync::now(device.clone())) as Box<dyn GpuFuture>);

}

fn window_size_dependent_setup(
    images: &[Arc<SwapchainImage<Window>>],
    render_pass: Arc<RenderPass>,
    viewport: &mut Viewport,
) -> Vec<Arc<Framebuffer>> {
    let dimensions = images[0].dimensions().width_height();
    viewport.dimensions = [dimensions[0] as f32, dimensions[1] as f32];

    images
        .iter()
        .map(|image| {
            let view = ImageView::new(image.clone()).unwrap();
            Framebuffer::start(render_pass.clone())
                .add(view)
                .unwrap()
                .build()
                .unwrap()
        })
        .collect::<Vec<_>>()
}