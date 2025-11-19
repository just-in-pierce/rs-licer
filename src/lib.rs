use bvh::aabb::{AABB, Bounded};
use bvh::bounding_hierarchy::BHShape;
use bvh::bvh::BVH;
use bvh::ray::Ray;
use glam::Vec3;
use rayon::prelude::*;
use std::fs::{self, File};
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Debug, Clone)]
pub struct SlicerConfig {
    pub input_path: String,
    pub output_dir: String,
    pub pixel_size_um: f32,
    pub layer_height_um: f32,
    pub zero_slice_position: bool,
    pub delete_below_zero: bool,
    pub delete_output_dir: bool,
    pub open_output_dir: bool,
}

#[derive(Debug, Clone, Copy)]
struct Triangle {
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
    node_index: usize,
}

impl Bounded for Triangle {
    fn aabb(&self) -> AABB {
        let min = self.v0.min(self.v1).min(self.v2);
        let max = self.v0.max(self.v1).max(self.v2);
        AABB::with_bounds(min, max)
    }
}

impl BHShape for Triangle {
    fn set_bh_node_index(&mut self, index: usize) {
        self.node_index = index;
    }

    fn bh_node_index(&self) -> usize {
        self.node_index
    }
}

impl Triangle {
    // Möller–Trumbore intersection algorithm
    fn intersect(&self, ray: &Ray) -> Option<f32> {
        let epsilon = 1e-6;
        let edge1 = self.v1 - self.v0;
        let edge2 = self.v2 - self.v0;
        let h = ray.direction.cross(edge2);
        let a = edge1.dot(h);

        if a > -epsilon && a < epsilon {
            return None;
        }

        let f = 1.0 / a;
        let s = ray.origin - self.v0;
        let u = f * s.dot(h);

        if u < 0.0 || u > 1.0 {
            return None;
        }

        let q = s.cross(edge1);
        let v = f * ray.direction.dot(q);

        if v < 0.0 || u + v > 1.0 {
            return None;
        }

        let t = f * edge2.dot(q);

        if t > epsilon {
            Some(t)
        } else {
            None
        }
    }
}

pub fn slice(config: SlicerConfig) {
    slice_with_progress(config, None);
}

pub fn slice_with_progress(config: SlicerConfig, progress_tx: Option<Sender<(f32, String)>>) {
    let pixel_size_mm = config.pixel_size_um / 1000.0;
    let layer_height_mm = config.layer_height_um / 1000.0;

    let send_progress = |progress: f32, message: &str| {
        if let Some(ref tx) = progress_tx {
            let _ = tx.send((progress, message.to_string()));
        }
    };

    send_progress(0.0, "Loading STL...");
    println!("Loading STL...");
    let mut file = File::open(&config.input_path).expect("Could not open input file");
    let mesh = stl_io::read_stl(&mut file).expect("Could not parse STL");
    
    let mut triangles = Vec::new();
    
    send_progress(0.05, &format!("Converting {} triangles...", mesh.faces.len()));
    println!("Converting {} triangles...", mesh.faces.len());
    for face in mesh.faces {
        let v0 = mesh.vertices[face.vertices[0]];
        let v1 = mesh.vertices[face.vertices[1]];
        let v2 = mesh.vertices[face.vertices[2]];
        
        triangles.push(Triangle {
            v0: Vec3::new(v0[0], v0[1], v0[2]),
            v1: Vec3::new(v1[0], v1[1], v1[2]),
            v2: Vec3::new(v2[0], v2[1], v2[2]),
            node_index: 0,
        });
    }

    send_progress(0.1, "Building BVH...");
    println!("Building BVH...");
    let bvh = BVH::build(&mut triangles);

    // Determine bounds
    let mut min_bound = Vec3::splat(f32::MAX);
    let mut max_bound = Vec3::splat(f32::MIN);

    for tri in &triangles {
        let aabb = tri.aabb();
        min_bound = min_bound.min(aabb.min);
        max_bound = max_bound.max(aabb.max);
    }

    println!("Bounds: Min {:?}, Max {:?}", min_bound, max_bound);

    let width_mm = max_bound.x - min_bound.x;
    let height_mm = max_bound.y - min_bound.y;
    
    let width_px = (width_mm / pixel_size_mm).ceil() as u32;
    let height_px = (height_mm / pixel_size_mm).ceil() as u32;
    
    println!("Image size: {} x {}", width_px, height_px);

    // Pre-calculate spans for each pixel
    send_progress(0.15, "Raytracing pixels...");
    println!("Raytracing pixels...");
    
    let bvh = &bvh;
    let triangles = &triangles;
    
    // We use a flattened vector for the grid
    let spans: Vec<Vec<(f32, f32)>> = (0..height_px).into_par_iter().flat_map(|y| {
        (0..width_px).into_par_iter().map(move |x| {
            let px = min_bound.x + (x as f32 + 0.5) * pixel_size_mm;
            let py = min_bound.y + (y as f32 + 0.5) * pixel_size_mm;
            
            // Ray from below the model pointing up
            let origin = Vec3::new(px, py, min_bound.z - 1.0);
            let direction = Vec3::new(0.0, 0.0, 1.0);
            let ray = Ray::new(origin, direction);
            
            let hit_shapes = bvh.traverse(&ray, &triangles);
            
            let mut hits: Vec<f32> = Vec::new();
            for shape in hit_shapes {
                if let Some(dist) = shape.intersect(&ray) {
                    // Convert distance to Z value
                    let z = origin.z + dist * direction.z;
                    hits.push(z);
                }
            }
            
            hits.sort_by(|a, b| a.partial_cmp(b).unwrap());
            
            // Create spans from pairs
            let mut pixel_spans = Vec::new();
            for i in (0..hits.len()).step_by(2) {
                if i + 1 < hits.len() {
                    pixel_spans.push((hits[i], hits[i+1]));
                }
            }
            pixel_spans
        })
    }).collect();

    // Generate images
    send_progress(0.5, "Generating slices...");
    println!("Generating slices...");
    
    // Delete output directory if requested
    if config.delete_output_dir && std::path::Path::new(&config.output_dir).exists() {
        fs::remove_dir_all(&config.output_dir).expect("Could not delete output directory");
    }
    
    fs::create_dir_all(&config.output_dir).expect("Could not create output directory");

    let start_z = min_bound.z;
    let end_z = max_bound.z;
    
    // Calculate number of layers
    let num_layers = ((end_z - start_z) / layer_height_mm).ceil() as u32;
    
    // Use atomic counter for thread-safe progress tracking
    let completed_layers = AtomicU32::new(0);
    let progress_tx_clone = progress_tx.clone();
    
    (0..num_layers).into_par_iter().for_each(|i| {
        let z = start_z + i as f32 * layer_height_mm;
        
        if config.delete_below_zero && z < 0.0 {
            return;
        }

        // Create image
        let mut img = image::GrayImage::new(width_px, height_px);
        
        for y in 0..height_px {
            for x in 0..width_px {
                let idx = (y * width_px + x) as usize;
                let pixel_spans = &spans[idx];
                
                let mut inside = false;
                // Add a small epsilon to handle floating point inaccuracies,
                // especially for flat surfaces aligned with the slice height.
                let epsilon = 1e-4; 
                for (enter, exit) in pixel_spans {
                    if z >= *enter - epsilon && z <= *exit + epsilon {
                        inside = true;
                        break;
                    }
                }
                
                if inside {
                    img.put_pixel(x, height_px - 1 - y, image::Luma([255]));
                } else {
                    img.put_pixel(x, height_px - 1 - y, image::Luma([0]));
                }
            }
        }
        
        let z_microns = if config.zero_slice_position {
            (i as f32 * config.layer_height_um).round() as i32
        } else {
            (z * 1000.0).round() as i32
        };
        let filename = format!("{}/{}.png", config.output_dir, z_microns);
        img.save(filename).expect("Could not save image");
        
        // Update progress after completing each layer
        let completed = completed_layers.fetch_add(1, Ordering::Relaxed) + 1;
        if completed % 5 == 0 || completed == num_layers {
            let progress = 0.5 + (completed as f32 / num_layers as f32) * 0.5;
            if let Some(ref tx) = progress_tx_clone {
                let _ = tx.send((progress, format!("Processing layer {} of {}", completed, num_layers)));
            }
        }
    });
    
    send_progress(1.0, "Done!");
    println!("Done!");
    
    // Open output directory if requested
    if config.open_output_dir {
        let _ = opener::open(&config.output_dir);
    }
}
