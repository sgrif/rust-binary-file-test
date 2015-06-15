extern crate byteorder;
use byteorder::{ReadBytesExt, LittleEndian};

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Result};
use std::rc::Rc;
use std::{mem, slice};

struct Mesh {
    vertices: Vec<f32>,
    uvs: Vec<f32>,
    normals: Vec<f32>,
    elements: HashMap<u32, Vec<u16>>,
    skin_weights: Vec<f32>,
    skin_indices: Vec<i16>,
}

struct Joint {
    parent: Option<Rc<RefCell<Joint>>>,
    name: String,
    rotation: (f32, f32, f32, f32),
    translation: (f32, f32, f32),
}

struct JointFileInfo {
    parent_idx: i16,
    name: String,
    rotation: (f32, f32, f32, f32),
    translation: (f32, f32, f32),
}

struct AnimationFile {
    meshes: Vec<Mesh>,
    skeleton: Vec<Rc<RefCell<Joint>>>,
    influences_per_vertex: i32,
}

fn main() {
    let mut f = File::open("model.anim").unwrap();
    let animation_file = parse_animation_file(&mut f).unwrap();
    println!("{}", animation_file.influences_per_vertex);
    let joint = animation_file.skeleton[0].borrow();
    println!("{}", joint.parent.as_ref().unwrap().borrow().name);
    println!("{}", joint.name);
    println!("{:?}", joint.rotation);
    println!("{:?}", joint.translation);
}

fn parse_animation_file<R: Read>(reader: &mut R) -> Result<AnimationFile> {
    let _version = try!(reader.read_i32::<LittleEndian>());
    let meshes = try!(ParseBinary::read(reader));

    let influences_per_vertex = try!(reader.read_i32::<LittleEndian>());
    let skeleton = try!(parse_skeleton(reader));

    Ok(AnimationFile {
        meshes: meshes,
        skeleton: skeleton,
        influences_per_vertex: influences_per_vertex,
    })
}

fn parse_skeleton<R: Read>(reader: &mut R) -> Result<Vec<Rc<RefCell<Joint>>>> {
    let joint_file_info: Vec<_> = try!(ParseBinary::read(reader));
    let mut result = vec![None; joint_file_info.len()];

    for i in (0..joint_file_info.len()) {
        build_joint(&joint_file_info, &mut result, i);
    }

    Ok(result.into_iter().map(|joint| joint.unwrap()).collect())
}

fn build_joint(infos: &Vec<JointFileInfo>, joints: &mut Vec<Option<Rc<RefCell<Joint>>>>, i: usize) {
    if joints[i].is_none() {
        let info = &infos[i];
        let parent;
        if info.parent_idx == -1 {
            parent = None;
        } else {
            build_joint(infos, joints, info.parent_idx as usize);
            parent = joints[info.parent_idx as usize].clone();
        }

        joints[i] = Some(Rc::new(RefCell::new(Joint {
            parent: parent,
            name: info.name.clone(),
            rotation: info.rotation,
            translation: info.translation,
        })));
    }
}

unsafe fn read_bytes<R: Read>(reader: &mut R, ptr: *mut u8, bytes_to_read: usize) -> Result<()> {
    let buf = slice::from_raw_parts_mut(ptr, bytes_to_read);
    let mut bytes_read = 0;
    while bytes_read < bytes_to_read {
        bytes_read += try!(reader.read(&mut buf[bytes_read..]));
    }
    Ok(())
}

fn read_primitive_array<T: Sized, R: Read>(reader: &mut R) -> Result<Vec<T>> {
    let num_elements = try!(reader.read_i32::<LittleEndian>()) as usize;
    let mut result = Vec::with_capacity(num_elements);
    unsafe {
        result.set_len(num_elements);
        let num_bytes = num_elements * mem::size_of::<T>();
        try!(read_bytes(reader, result.as_mut_ptr() as *mut u8, num_bytes));
    }
    Ok(result)
}

fn read_faces<R: Read>(reader: &mut R) -> Result<HashMap<u32, Vec<u16>>> {
    let num_face_sets = try!(reader.read_i32::<LittleEndian>());
    (0..num_face_sets).map(|_i| {
        let material_index = try!(reader.read_u32::<LittleEndian>());
        let quads: Vec<u16> = try!(read_primitive_array(reader));
        let mut triangles: Vec<u16> = try!(read_primitive_array(reader));
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
    }).collect()
}

trait ParseBinary {
    fn read<R: Read>(reader: &mut R) -> Result<Self>;
}

impl ParseBinary for Mesh {
    fn read<R: Read>(reader: &mut R) -> Result<Mesh> {
        let vertices = try!(read_primitive_array(reader));
        let short_uvs: Vec<i16> = try!(read_primitive_array(reader));
        let uvs = short_uvs.iter().map(|&uv| (uv as f32) / 4096.0).collect();
        let normals = try!(read_primitive_array(reader));
        let faces = try!(read_faces(reader));
        let skin_weights = try!(read_primitive_array(reader));
        let skin_indices = try!(read_primitive_array(reader));

        Ok(Mesh {
            vertices: vertices,
            uvs: uvs,
            normals: normals,
            elements: faces,
            skin_weights: skin_weights,
            skin_indices: skin_indices,
        })
    }
}

impl<T> ParseBinary for Vec<T> where T: ParseBinary {
    fn read<R: Read>(reader: &mut R) -> Result<Vec<T>> {
        let num_elements = try!(reader.read_i32::<LittleEndian>());
        (0..num_elements).map(|_i| {
            ParseBinary::read(reader)
        }).collect()
    }
}

impl ParseBinary for JointFileInfo {
    fn read<R: Read>(reader: &mut R) -> Result<JointFileInfo> {
        let parent_idx = try!(reader.read_i16::<LittleEndian>());
        let name = try!(ParseBinary::read(reader));
        let rotation = try!(ParseBinary::read(reader));
        let translation = try!(ParseBinary::read(reader));
        Ok(JointFileInfo {
            parent_idx: parent_idx,
            name: name,
            rotation: rotation,
            translation: translation,
        })
    }
}

impl ParseBinary for String {
    fn read<R: Read>(reader: &mut R) -> Result<String> {
        let length = try!(reader.read_u16::<LittleEndian>()) as usize;
        let mut bytes = Vec::with_capacity(length);
        unsafe {
            bytes.set_len(length);
            try!(read_bytes(reader, bytes.as_mut_ptr() as *mut u8, length));
            Ok(String::from_utf8_unchecked(bytes))
        }
    }
}

trait ParseAsRawBytes {}
impl<A: ParseAsRawBytes + Sized> ParseBinary for A {
    fn read<R: Read>(reader: &mut R) -> Result<Self> {
        let bytes_to_read = mem::size_of::<Self>();
        let result: Self;
        unsafe {
            result = mem::uninitialized();
            try!(read_bytes(reader, mem::transmute(&result), bytes_to_read));
        }
        Ok(result)
    }
}

impl<A, B, C> ParseAsRawBytes for (A, B, C) {}
impl<A, B, C, D> ParseAsRawBytes for (A, B, C, D) {}
