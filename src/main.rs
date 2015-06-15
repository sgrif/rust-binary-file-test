extern crate byteorder;

use byteorder::{ReadBytesExt, LittleEndian};

use std::io;
use std::slice;
use std::fs::File;
use std::mem;
use std::collections::HashMap;

struct Mesh {
    vertices: Vec<f32>,
    uvs: Vec<f32>,
    normals: Vec<f32>,
    elements: HashMap<u32, Vec<u16>>,
    skin_weights: Vec<f32>,
    skin_indices: Vec<i16>,
}

struct AnimationFile {
    meshes: Vec<Mesh>,
}

fn main() {
    let mut f = File::open("model.anim").unwrap();
    let animation_file;
    unsafe { animation_file = parse_animation_file(&mut f); }
    println!("[{}, {}, {}]", animation_file.meshes[0].vertices[0], animation_file.meshes[0].vertices[1], animation_file.meshes[0].vertices[2]);
    println!("[{}, {}]", animation_file.meshes[0].uvs[0], animation_file.meshes[0].uvs[1]);
    println!("[{}, {}, {}]", animation_file.meshes[0].normals[0], animation_file.meshes[0].normals[1], animation_file.meshes[0].normals[2]);
    println!("[{}, {}, {}]", animation_file.meshes[0].skin_weights[0], animation_file.meshes[0].skin_weights[1], animation_file.meshes[0].skin_weights[2]);
    println!("[{}, {}, {}]", animation_file.meshes[0].skin_indices[0], animation_file.meshes[0].skin_indices[1], animation_file.meshes[0].skin_indices[2]);
}

unsafe fn parse_animation_file<R: io::Read>(reader: &mut R) -> AnimationFile {
    let _version = reader.read_i32::<LittleEndian>().unwrap();
    let num_meshes = reader.read_i32::<LittleEndian>().unwrap();
    let meshes = (0..num_meshes).map(|_i| {
        parse_mesh(reader)
    }).collect();
    AnimationFile { meshes: meshes }
}

unsafe fn parse_mesh<R: io::Read>(reader: &mut R) -> Mesh {
    let vertices = read_array(reader);
    let short_uvs: Vec<i16> = read_array(reader);
    let uvs = short_uvs.iter().map(|&uv| (uv as f32) / 4096.0).collect();
    let normals = read_array(reader);
    let faces = read_faces(reader);
    let skin_weights = read_array(reader);
    let skin_indices = read_array(reader);

    Mesh {
        vertices: vertices,
        uvs: uvs,
        normals: normals,
        elements: faces,
        skin_weights: skin_weights,
        skin_indices: skin_indices,
    }
}

unsafe fn read_array<T: Sized, R: io::Read>(reader: &mut R) -> Vec<T> {
    let num_elements = reader.read_i32::<LittleEndian>().unwrap() as usize;
    let mut result = Vec::with_capacity(num_elements);
    result.set_len(num_elements);
    let num_bytes = num_elements * mem::size_of::<T>();
    read_bytes(reader, result.as_mut_ptr() as *mut u8, num_bytes);
    result
}

unsafe fn read_bytes<R: io::Read>(reader: &mut R, ptr: *mut u8, bytes_to_read: usize) {
    let buf = slice::from_raw_parts_mut(ptr, bytes_to_read);
    let mut bytes_read = 0;
    while bytes_read < bytes_to_read {
        bytes_read += reader.read(&mut buf[bytes_read..]).unwrap();
    }
}

unsafe fn read_faces<R: io::Read>(reader: &mut R) -> HashMap<u32, Vec<u16>> {
    let num_face_sets = reader.read_i32::<LittleEndian>().unwrap();
    (0..num_face_sets).map(|_i| {
        let material_index = reader.read_u32::<LittleEndian>().unwrap();
        let quads: Vec<u16> = read_array(reader);
        let mut triangles: Vec<u16> = read_array(reader);
        triangles.reserve((quads.len() as f32 * 1.5) as usize);

        // I'm sure there's an iterator for this but I can't seem to find it.
        let mut i = 0;
        while i < quads.len() {
            triangles.push(quads[i + 0]);
            triangles.push(quads[i + 1]);
            triangles.push(quads[i + 3]);
            triangles.push(quads[i + 1]);
            triangles.push(quads[i + 2]);
            triangles.push(quads[i + 3]);
            i += 4;
        }

        (material_index, triangles)
    }).collect()
}
