use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    Performance,
    WebGl2RenderingContext,
    WebGlProgram,
    WebGlShader,
    WebGlUniformLocation,
};

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let window = web_sys::window().expect("should have a window in this context");
    let document = window.document().expect("should have a document in this context");
    let canvas = document.get_element_by_id("canvas").unwrap();
    let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>()?;
    let body = document.body().unwrap().dyn_into::<web_sys::HtmlBodyElement>()?;

    let performance = window
        .performance()
        .expect("performance should be available");

    let boot_time = perf_to_system(performance.now());

    canvas.set_width(body.client_width() as u32);
    canvas.set_height(body.client_height() as u32);

    let context = canvas
        .get_context("webgl2")?
        .unwrap()
        .dyn_into::<WebGl2RenderingContext>()?;

    let vert_shader = compile_shader(
        &context,
        WebGl2RenderingContext::VERTEX_SHADER,
        include_str!("shaders/vert.glsl"),
    )?;

    let frag_shader = compile_shader(
        &context,
        WebGl2RenderingContext::FRAGMENT_SHADER,
        include_str!("shaders/frag.glsl"),
    )?;
    let program = link_program(&context, &vert_shader, &frag_shader)?;
    context.use_program(Some(&program));

    let vertices: [f32; 18] = [
        -1.0, -1.0, 0.0,
        -1.0, 1.0, 0.0,
        1.0, 1.0, 0.0,

        1.0, 1.0, 0.0,
        1.0, -1.0, 0.0,
        -1.0, -1.0, 0.0,
    ];
    let vert_count = (vertices.len() / 3) as i32;

    let position_attribute_location = context.get_attrib_location(&program, "position");
    let buffer = context.create_buffer().ok_or("Failed to create buffer")?;
    context.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&buffer));

    // Note that `Float32Array::view` is somewhat dangerous (hence the
    // `unsafe`!). This is creating a raw view into our module's
    // `WebAssembly.Memory` buffer, but if we allocate more pages for ourself
    // (aka do a memory allocation in Rust) it'll cause the buffer to change,
    // causing the `Float32Array` to be invalid.
    //
    // As a result, after `Float32Array::view` we have to be very careful not to
    // do any memory allocations before it's dropped.
    unsafe {
        let positions_array_buf_view = js_sys::Float32Array::view(&vertices);

        context.buffer_data_with_array_buffer_view(
            WebGl2RenderingContext::ARRAY_BUFFER,
            &positions_array_buf_view,
            WebGl2RenderingContext::STATIC_DRAW,
        );
    }

    let vao = context
        .create_vertex_array()
        .ok_or("Could not create vertex array object")?;
    context.bind_vertex_array(Some(&vao));

    context.vertex_attrib_pointer_with_i32(0, 3, WebGl2RenderingContext::FLOAT, false, 0, 0);
    context.enable_vertex_attrib_array(position_attribute_location as u32);

    context.bind_vertex_array(Some(&vao));

    let f = Rc::new(RefCell::new(None));
    let g = f.clone();

    let mut frame = 0;
    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        context.clear_color(0.0, 0.0, 0.0, 1.0);
        context.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);

        set_uniforms(
            &context,
            &body,
            &performance,
            &program,
        );

        context.draw_arrays(
            WebGl2RenderingContext::TRIANGLES,
            0,
            vert_count
        );

        frame += 1;

        request_animation_frame(f.borrow().as_ref().unwrap());
    }) as Box<dyn FnMut()>));

    request_animation_frame(g.borrow().as_ref().unwrap());

    Ok(())
}

fn set_uniforms(
    context: &WebGl2RenderingContext,
    body: &web_sys::HtmlBodyElement,
    performance: &Performance,
    program: &WebGlProgram,
) {
    let u_resolution = context.get_uniform_location(&program, "u_resolution");
    let u_time = context.get_uniform_location(&program, "u_time");

    let mut u_res_val = [body.client_width() as f32, body.client_height() as f32];
    context.uniform2fv_with_f32_array(u_resolution.as_ref(), &mut u_res_val);

    let start = perf_to_system(performance.now());
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    context.uniform1f(u_time.as_ref(), since_the_epoch.as_secs_f32());
}

fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    web_sys::window().expect("no global `window` exists")
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("should register `requestAnimationFrame` OK");
}

pub fn compile_shader(
    context: &WebGl2RenderingContext,
    shader_type: u32,
    source: &str,
) -> Result<WebGlShader, String> {
    let shader = context
        .create_shader(shader_type)
        .ok_or_else(|| String::from("Unable to create shader object"))?;
    context.shader_source(&shader, source);
    context.compile_shader(&shader);

    if context
        .get_shader_parameter(&shader, WebGl2RenderingContext::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        Err(context
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| String::from("Unknown error creating shader")))
    }
}

pub fn link_program(
    context: &WebGl2RenderingContext,
    vert_shader: &WebGlShader,
    frag_shader: &WebGlShader,
) -> Result<WebGlProgram, String> {
    let program = context
        .create_program()
        .ok_or_else(|| String::from("Unable to create shader object"))?;

    context.attach_shader(&program, vert_shader);
    context.attach_shader(&program, frag_shader);
    context.link_program(&program);

    if context
        .get_program_parameter(&program, WebGl2RenderingContext::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        Err(context
            .get_program_info_log(&program)
            .unwrap_or_else(|| String::from("Unknown error creating program object")))
    }
}

fn perf_to_system(amt: f64) -> SystemTime {
    let secs = (amt as u64) / 1_000;
    let nanos = (((amt as u64) % 1_000) as u32) * 1_000_000;
    UNIX_EPOCH + Duration::new(secs, nanos)
}
