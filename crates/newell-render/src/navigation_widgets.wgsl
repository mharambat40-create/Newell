struct Uniforms {
 right:vec4<f32>, up:vec4<f32>, forward:vec4<f32>, group:vec4<f32>,
 axis:vec4<f32>, cube:vec4<f32>, flags:vec4<u32>, surface_atlas:vec4<f32>,
}
@group(0) @binding(0) var<uniform> u:Uniforms;
@group(0) @binding(1) var<storage,read> style:array<vec4<f32>>;
@group(0) @binding(2) var words:texture_2d<f32>;
@group(0) @binding(3) var word_sampler:sampler;
@group(0) @binding(4) var<storage,read> clips:array<vec4<f32>>;
fn linear(c:vec3<f32>)->vec3<f32> {return select(pow((c+vec3<f32>(0.055))/1.055,vec3<f32>(2.4)),c/12.92,c<=vec3<f32>(0.04045));}
fn encoded(c:vec3<f32>)->vec3<f32> {return select(1.055*pow(max(c,vec3<f32>(0.0)),vec3<f32>(1.0/2.4))-vec3<f32>(0.055),12.92*c,c<=vec3<f32>(0.0031308));}
fn corner(i:u32)->vec2<f32> {var c=array<vec2<f32>,6>(vec2<f32>(0,0),vec2<f32>(1,0),vec2<f32>(0,1),vec2<f32>(0,1),vec2<f32>(1,0),vec2<f32>(1,1));return c[i];}
fn ndc(p:vec2<f32>)->vec4<f32> {return vec4<f32>(2.0*p.x/u.group.z-1.0,1.0-2.0*p.y/u.group.w,0.0,1.0);}
fn over(front:vec4<f32>,back:vec4<f32>)->vec4<f32> {return front+back*(1.0-front.a);}
struct CubeIn {@location(0) pos:vec3<f32>,@location(1) component_id:u32,@location(2) normal:vec3<f32>,@location(3) material:u32,@location(4) seam:vec4<f32>}
struct CubeOut {@builtin(position) pos:vec4<f32>,@location(0) normal:vec3<f32>,@location(1) @interpolate(flat) material:u32,@location(2) @interpolate(flat) component_id:u32}
fn cube_screen(v:vec3<f32>)->vec2<f32> {return u.cube.xy+u.cube.z*vec2<f32>(dot(u.right.xyz,v),-dot(u.up.xyz,v));}
fn cube_projection(v:vec3<f32>)->vec4<f32> {var p=ndc(cube_screen(v));p.z=0.5+dot(u.forward.xyz,v)*0.4;return p;}
@vertex fn vs_cube(v:CubeIn)->CubeOut {
 var o:CubeOut;o.pos=cube_projection(v.pos);
 o.normal=vec3<f32>(dot(u.right.xyz,v.normal),dot(u.up.xyz,v.normal),-dot(u.forward.xyz,v.normal));o.material=v.material;o.component_id=v.component_id;return o;
}
@fragment fn fs_cube(v:CubeOut)->@location(0) vec4<f32> {
 if v.material>0u&&u.flags.x!=v.component_id+1u {discard;}
 let light=0.88+0.12*max(0.0,dot(normalize(v.normal),normalize(vec3<f32>(-0.4,0.6,1.0))));
 return vec4<f32>(linear(style[3u+min(v.material,2u)].rgb)*light,1.0);
}
struct LabelIn {@location(0) origin:vec4<f32>,@location(1) horizontal:vec4<f32>,@location(2) vertical:vec4<f32>,@location(3) uv:vec4<f32>,@location(4) clip:vec4<u32>,@location(5) color:vec4<f32>}
struct LabelOut {@builtin(position) pos:vec4<f32>,@location(0) xy:vec2<f32>,@location(1) uv:vec2<f32>,@location(2) @interpolate(flat) clip:vec4<u32>,@location(3) color:vec4<f32>}
@vertex fn vs_label(v:LabelIn,@builtin(vertex_index) i:u32)->LabelOut {
 let c=corner(i);let local=v.origin.xyz+(c.x-0.5)*v.horizontal.xyz*v.horizontal.w+(0.5-c.y)*v.vertical.xyz*v.vertical.w;var o:LabelOut;o.xy=cube_screen(local);o.pos=cube_projection(local);o.uv=v.uv.xy+c*v.uv.zw;o.clip=v.clip;o.color=v.color;return o;
}
@fragment fn fs_label(v:LabelOut)->@location(0) vec4<f32> {
 var positive=false;var negative=false;
 for(var i=0u;i<v.clip.y;i++) {let a=clips[v.clip.x+i].xy;let b=clips[v.clip.x+(i+1u)%v.clip.y].xy;let d=b-a;let p=v.xy-a;let c=d.x*p.y-d.y*p.x;positive=positive||c>0.0001;negative=negative||c< -0.0001;}
 if positive&&negative {discard;}
 let alpha=textureSampleLevel(words,word_sampler,v.uv,0.0).r;return vec4<f32>(linear(v.color.rgb)*alpha,alpha);
}
// Composite uses a separate bind layout: resolved color occupies word texture binding.
struct CompositeOut {@builtin(position) pos:vec4<f32>,@location(0) local:vec2<f32>}
@vertex fn vs_composite(@builtin(vertex_index) i:u32)->CompositeOut {let c=corner(i);let p=u.group.xy+c*u.group.zw;var o:CompositeOut;o.pos=vec4<f32>(2.0*p.x/u.surface_atlas.x-1.0,1.0-2.0*p.y/u.surface_atlas.y,0.,1.);o.local=c;return o;}
@fragment fn fs_composite(v:CompositeOut)->@location(0) vec4<f32> {
 let p=v.local*u.group.zw+u.group.xy;
 for(var i=0u;i<u.flags.w;i++) {let r=clips[i];if all(p>=r.xy)&&all(p<r.zw) {discard;}}
 var c=textureSampleLevel(words,word_sampler,v.local,0.0);
 if u.flags.z==0u&&c.a>0.0 {c=vec4<f32>(encoded(c.rgb/c.a)*c.a,c.a);}return c;
}
