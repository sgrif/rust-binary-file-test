extern crate byteorder;

use byteorder::{ReadBytesExt, LittleEndian};

use std::io::{Read, Result};
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
    unsafe { animation_file = parse_animation_file(&mut f).unwrap(); }
    println!("[{}, {}, {}]", animation_file.meshes[0].vertices[0], animation_file.meshes[0].vertices[1], animation_file.meshes[0].vertices[2]);
    println!("[{}, {}]", animation_file.meshes[0].uvs[0], animation_file.meshes[0].uvs[1]);
    println!("[{}, {}, {}]", animation_file.meshes[0].normals[0], animation_file.meshes[0].normals[1], animation_file.meshes[0].normals[2]);
    println!("[{}, {}, {}]", animation_file.meshes[0].skin_weights[0], animation_file.meshes[0].skin_weights[1], animation_file.meshes[0].skin_weights[2]);
    println!("[{}, {}, {}]", animation_file.meshes[0].skin_indices[0], animation_file.meshes[0].skin_indices[1], animation_file.meshes[0].skin_indices[2]);
}

unsafe fn parse_animation_file<R: Read>(reader: &mut R) -> Result<AnimationFile> {
    let _version = try!(reader.read_i32::<LittleEndian>());
    let num_meshes = try!(reader.read_i32::<LittleEndian>());;
    let meshes = try!((0..num_meshes).map(|_i| {
        parse_mesh(reader)
    }).collect());
    Ok(AnimationFile { meshes: meshes })
}

unsafe fn parse_mesh<R: Read>(reader: &mut R) -> Result<Mesh> {
    let vertices = try!(read_array(reader));
    let short_uvs: Vec<i16> = try!(read_array(reader));
    let uvs = short_uvs.iter().map(|&uv| (uv as f32) / 4096.0).collect();
    let normals = try!(read_array(reader));
    let faces = try!(read_faces(reader));
    let skin_weights = try!(read_array(reader));
    let skin_indices = try!(read_array(reader));

    Ok(Mesh {
        vertices: vertices,
        uvs: uvs,
        normals: normals,
        elements: faces,
        skin_weights: skin_weights,
        skin_indices: skin_indices,
    })
}

unsafe fn read_array<T: Sized, R: Read>(reader: &mut R) -> Result<Vec<T>> {
    let num_elements = try!(reader.read_i32::<LittleEndian>()) as usize;
    let mut result = Vec::with_capacity(num_elements);
    result.set_len(num_elements);
    let num_bytes = num_elements * mem::size_of::<T>();
    try!(read_bytes(reader, result.as_mut_ptr() as *mut u8, num_bytes));
    Ok(result)
}

unsafe fn read_bytes<R: Read>(reader: &mut R, ptr: *mut u8, bytes_to_read: usize) -> Result<()> {
    let buf = slice::from_raw_parts_mut(ptr, bytes_to_read);
    let mut bytes_read = 0;
    while bytes_read < bytes_to_read {
        bytes_read += try!(reader.read(&mut buf[bytes_read..]));
    }
    Ok(())
}

unsafe fn read_faces<R: Read>(reader: &mut R) -> Result<HashMap<u32, Vec<u16>>> {
    let num_face_sets = try!(reader.read_i32::<LittleEndian>());
    (0..num_face_sets).map(|_i| {
        let material_index = try!(reader.read_u32::<LittleEndian>());
        let quads: Vec<u16> = try!(read_array(reader));
        let mut triangles: Vec<u16> = try!(read_array(reader));
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

        Ok((material_index, triangles))
    }).collect::<Result<HashMap<u32, Vec<u16>>>>()
}
