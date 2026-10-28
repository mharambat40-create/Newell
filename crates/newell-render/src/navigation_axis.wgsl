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
struct AxisOut {@builtin(position) pos:vec4<f32>,@location(0) xy:vec2<f32>}
@vertex fn vs_axis(@builtin(vertex_index) i:u32)->AxisOut {var o:AxisOut;o.xy=corner(i)*u.group.zw;o.pos=ndc(o.xy);return o;}
fn coverage(d:f32)->f32 {return clamp(0.5-d,0.0,1.0);}
@fragment fn fs_axis(v:AxisOut)->@location(0) vec4<f32> {
 let q=u.axis.z;let p=v.xy-u.axis.xy;let scale=u.axis.w;
 var result=vec4<f32>(0.0);
 // The world-origin sphere and reference planes render in Scene. The corner widget
 // retains only its positive direction-control stems and all six caps/rings.
 for(var axis=0u;axis<3u;axis++) {
   let end=scale*vec2<f32>(u.right[axis],-u.up[axis]);let len2=dot(end,end);
   if len2>0.0000001 {
     let t=clamp(dot(p,end)/len2,0.0,1.0);let distance=length(p-end*t);
     let alpha=coverage(distance-max(style[26].x,style[26].x*q)*0.5)*mix(style[26].y,style[26].z,t);
     result=over(vec4<f32>(linear(style[axis].rgb)*alpha,alpha),result);
   }
 }
 // Direction caps stay screen-facing and synchronized to the scene camera.
 for(var c=0u;c<6u;c++) {
   let n=style[18u+c].xyz;let axis=select(select(2u,1u,abs(n.y)>0.5),0u,abs(n.x)>0.5);let positive=n[axis]>0.0;let sign=n[axis];
   let center=scale*sign*vec2<f32>(u.right[axis],-u.up[axis]);let delta=p-center;let d=length(delta);
   let fill=style[28u+c*3u];let stroke=style[29u+c*3u];let letter=style[30u+c*3u];
   var marker=vec4<f32>(0.0);
   if positive {
     let outer=coverage(d-style[24].y*q);
     let inner=coverage(d-(style[24].y-style[24].w)*q);
     let outline=max(0.0,outer-inner);
     marker=vec4<f32>(linear(fill.rgb)*fill.a*inner+linear(stroke.rgb)*stroke.a*outline,fill.a*inner+stroke.a*outline);
     let bounds=style[12u+axis];let size=style[15u+axis].xy;let pos=delta/size+vec2<f32>(0.5);
     if all(pos>=vec2<f32>(0.0))&&all(pos<=vec2<f32>(1.0)) {
       let ink=textureSampleLevel(words,word_sampler,bounds.xy+pos*bounds.zw,0.0).r*letter.a*inner;
       marker=over(vec4<f32>(linear(letter.rgb)*ink,ink),marker);
     }
     // A hollow positive marker reveals the actual scene, including where its
     // own stem or a rear marker would otherwise show through the opening.
     if fill.a==0.0 {result*=1.0-outer;}
   } else {
     let alpha=coverage(abs(d-style[24].z*q)-0.5*style[24].w*q)*stroke.a;
     marker=vec4<f32>(linear(stroke.rgb)*alpha,alpha);
   }
   result=over(marker,result);
 }
 return result;
}
