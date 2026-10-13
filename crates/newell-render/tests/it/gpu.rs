// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

/// Manual: `cargo nextest run -p newell-render --run-ignored all` on a machine with a GPU.
/// The window path is verified manually with `cargo run --release`.
#[test]
#[ignore = "needs a GPU"]
fn device_is_created_headless() {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = pollster_free(instance.request_adapter(&wgpu::RequestAdapterOptions::default()));
    let adapter = adapter.unwrap();
    pollster_free(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
}

fn pollster_free<F: std::future::Future>(f: F) -> F::Output {
    use std::sync::Arc;
    use std::task::{Context, Poll, Wake, Waker};
    struct W(std::thread::Thread);
    impl Wake for W {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }
    let waker = Waker::from(Arc::new(W(std::thread::current())));
    let mut cx = Context::from_waker(&waker);
    let mut f = std::pin::pin!(f);
    loop {
        match f.as_mut().poll(&mut cx) {
            Poll::Ready(v) => return v,
            Poll::Pending => std::thread::park(),
        }
    }
}
