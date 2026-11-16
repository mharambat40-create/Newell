// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
struct Vertex {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

@vertex fn vs(@location(0) position: vec2<f32>, @location(1) color: vec3<f32>) -> Vertex {
    return Vertex(vec4<f32>(position, 0.0, 1.0), color);
}

@fragment fn fs(input: Vertex) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}
