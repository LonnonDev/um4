struct Engine {
    event_loop: EventLoop<()>
}

impl Engine {
    pub fn new() {
        
    }

    pub fn control_loop() {
        self.event_loop.run(move |event, _, control_flow| {
            *control_flow = match event {
                glutin::event::Event::WindowEvent { event, .. } => match event {
                    // Break from the main loop when the window is closed.
                    glutin::event::WindowEvent::CloseRequested => glutin::event_loop::ControlFlow::Exit,
                    // Redraw the triangle when the window is resized.
                    glutin::event::WindowEvent::Resized(..) => {
                        draw();
                        glutin::event_loop::ControlFlow::Poll
                    },
                    _ => glutin::event_loop::ControlFlow::Poll,
                },
                _ => glutin::event_loop::ControlFlow::Poll,
            };
        });
    }
}