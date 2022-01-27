use std::{sync::Arc, cell::RefCell};

use vulkano::{instance::Instance, Version, device::{physical::PhysicalDevice, DeviceExtensions, Device, Features}, swapchain::{Swapchain, SwapchainCreationError, AcquireError, self}, pipeline::graphics::viewport::Viewport, image::{SwapchainImage, ImageAccess, view::ImageView}, render_pass::{RenderPass, Framebuffer}, sync::{self, GpuFuture, FlushError}, command_buffer::{AutoCommandBufferBuilder, SubpassContents, CommandBufferUsage}};
use vulkano_win::VkSurfaceBuild;
use winit::{event_loop::{EventLoop, ControlFlow}, window::{WindowBuilder, Window}, platform::unix::EventLoopExtUnix, event::{Event, WindowEvent}};


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

    

    //main loop
    event_loop.run(move |event, _, control_flow| {
        //release any resources when the gpu is done
        previous_frame_end.take().unwrap().cleanup_finished();

        //recreate the swapchain if requested
        if recreate_swapchain {
            // dimensions of the window
            let dimensions = surface.window().inner_size().into();
            let (new_swapchain, new_images) =
                match swapchain.recreate().dimensions(dimensions).build() {
                    Ok(r) => r,
                    //generally means the window is being resized by the user, so it is ignored
                    Err(SwapchainCreationError::UnsupportedDimensions) => return,
                    Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
                };
        
            swapchain = new_swapchain;
            framebuffers = window_size_dependent_setup(
                &new_images,
                render_pass.clone(),
                &mut viewport,
            );
            recreate_swapchain = false;
        }

        //handle various events
        match event {
            //close window
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            },
            //resize window
            Event::WindowEvent { event: WindowEvent::Resized(_), .. } => {
                recreate_swapchain = true;
            },
            //all clear
            Event::RedrawEventsCleared => {
                // do our render operations here

                //get the next image from the swapchain
                let (image_num, suboptimal, acquire_future) = match swapchain::acquire_next_image(swapchain.clone(), None) {
                    Ok(r) => r,
                    Err(AcquireError::OutOfDate) => {
                        recreate_swapchain = true;
                        return;
                    },
                    Err(e) => panic!("Failed to acquire next image: {:?}", e)
                };
                //if the swapchain settings are "suboptimal" according to vulkan, recreate the swapchain
                if suboptimal {
                    recreate_swapchain = true;
                }

                let clear_values = vec!([0.0, 0.0, 0.0, 1.0].into());

                //create the command buffer builder
                let mut cmd_buffer_builder = AutoCommandBufferBuilder::primary(
                    device.clone(), queue.family(), CommandBufferUsage::OneTimeSubmit).unwrap();
                cmd_buffer_builder
                    .begin_render_pass(framebuffers[image_num].clone(), SubpassContents::Inline, clear_values.clone())
                    .unwrap()
                    .end_render_pass()
                    .unwrap();
                //create the command buffer
                let command_buffer = cmd_buffer_builder.build().unwrap();

                //make sure we aren't rendering before everything else is done
                let future = previous_frame_end.join(acquire_future)
                    .then_execute(queue.clone(), command_buffer).unwrap()
                    .then_swapchain_present(queue.clone(), swapchain.clone(), image_num)
                    .then_signal_fence_and_flush();
                
                //checking up on what our graphics hardware is doing :)
                match future {
                    Ok(future) => {
                        previous_frame_end = Some(Box::new(future) as Box<_>);
                    }
                    Err(FlushError::OutOfDate) => {
                        recreate_swapchain = true;
                        previous_frame_end = Some(Box::new(sync::now(device.clone())) as Box<_>);
                    }
                    Err(e) => {
                        println!("Failed to flush future: {:?}", e);
                        previous_frame_end = Some(Box::new(sync::now(device.clone())) as Box<_>);
                    }
                }
            },
            _ => {}
        }
    });
        

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