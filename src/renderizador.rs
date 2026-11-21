//! Renderizador software para os meshes do egui.
//!
//! Substitui o glow/OpenGL: rasteriza os triângulos texturizados que o egui
//! produz em um buffer de pixels, sem nenhuma dependência de GPU. O custo
//! de memória cai de ~140 MB (contexto OpenGL + atlas) para ~5 MB (buffer +
//! atlas em RAM).

use std::collections::HashMap;

use egui::epaint::{ClippedPrimitive, Primitive, Vertex};
use egui::{Color32, Rect, TextureId, TexturesDelta};

struct Textura {
    largura: u32,
    altura: u32,
    pixels: Vec<Color32>,
}

impl Textura {
    fn amostra(&self, u: f32, v: f32) -> Color32 {
        let fx = u * self.largura as f32 - 0.5;
        let fy = v * self.altura as f32 - 0.5;

        let x0 = fx.floor() as i32;
        let y0 = fy.floor() as i32;

        let sx = fx - x0 as f32;
        let sy = fy - y0 as f32;

        let lim_x = self.largura as i32 - 1;
        let lim_y = self.altura as i32 - 1;
        let x0c = x0.clamp(0, lim_x) as u32;
        let y0c = y0.clamp(0, lim_y) as u32;
        let x1c = (x0 + 1).clamp(0, lim_x) as u32;
        let y1c = (y0 + 1).clamp(0, lim_y) as u32;

        let w = self.largura;
        let c00 = self.pixels[(y0c * w + x0c) as usize];
        let c10 = self.pixels[(y0c * w + x1c) as usize];
        let c01 = self.pixels[(y1c * w + x0c) as usize];
        let c11 = self.pixels[(y1c * w + x1c) as usize];

        let inv_sx = 1.0 - sx;
        let inv_sy = 1.0 - sy;

        let r = inv_sy * (inv_sx * c00.r() as f32 + sx * c10.r() as f32)
              +     sy * (inv_sx * c01.r() as f32 + sx * c11.r() as f32);
        let g = inv_sy * (inv_sx * c00.g() as f32 + sx * c10.g() as f32)
              +     sy * (inv_sx * c01.g() as f32 + sx * c11.g() as f32);
        let b = inv_sy * (inv_sx * c00.b() as f32 + sx * c10.b() as f32)
              +     sy * (inv_sx * c01.b() as f32 + sx * c11.b() as f32);
        let a = inv_sy * (inv_sx * c00.a() as f32 + sx * c10.a() as f32)
              +     sy * (inv_sx * c01.a() as f32 + sx * c11.a() as f32);

        Color32::from_rgba_unmultiplied(r as u8, g as u8, b as u8, a as u8)
    }
}

pub struct Renderizador {
    largura: u32,
    altura: u32,
    buffer: Vec<u32>,
    texturas: HashMap<TextureId, Textura>,
}

impl Renderizador {
    pub fn novo(largura: u32, altura: u32) -> Self {
        Self {
            largura,
            altura,
            buffer: vec![0; (largura * altura) as usize],
            texturas: HashMap::new(),
        }
    }

    pub fn redimensionar(&mut self, largura: u32, altura: u32) {
        if self.largura != largura || self.altura != altura {
            self.largura = largura;
            self.altura = altura;
            self.buffer.resize((largura * altura) as usize, 0);
        }
    }

    pub fn aplicar_texturas(&mut self, delta: &TexturesDelta) {
        for (id, deltas) in &delta.set {
            for img_delta in deltas {
                let imagem = &img_delta.image;
                let pixels: Vec<Color32> = match imagem {
                    egui::ImageData::Color(c) => c.pixels.clone(),
                };
                if let Some(pos) = img_delta.pos {
                    if let Some(tex) = self.texturas.get_mut(id) {
                        let w = imagem.width();
                        for y in 0..imagem.height() {
                            for x in 0..w {
                                let dst_x = pos[0] + x;
                                let dst_y = pos[1] + y;
                                if dst_x < tex.largura as usize && dst_y < tex.altura as usize {
                                    tex.pixels[dst_y * tex.largura as usize + dst_x] =
                                        pixels[y * w + x];
                                }
                            }
                        }
                    }
                } else {
                    self.texturas.insert(
                        *id,
                        Textura {
                            largura: imagem.width() as u32,
                            altura: imagem.height() as u32,
                            pixels,
                        },
                    );
                }
            }
        }
        for id in &delta.free {
            self.texturas.remove(id);
        }
    }

    pub fn renderizar(&mut self, primitivas: &[ClippedPrimitive], escala: f32) {
        self.buffer.fill(0);
        for prim in primitivas {
            if let Primitive::Mesh(ref mesh) = prim.primitive {
                self.rasterizar_mesh(mesh, prim.clip_rect, escala);
            }
        }
    }

    pub fn pixels(&self) -> &[u32] {
        &self.buffer
    }

    fn rasterizar_mesh(&mut self, mesh: &egui::epaint::Mesh, clip: Rect, escala: f32) {
        let textura = self.texturas.get(&mesh.texture_id);
        let indices = &mesh.indices;
        let verts = &mesh.vertices;
        let (w, h) = (self.largura as i32, self.altura as i32);

        let clip_min_x = (clip.min.x * escala) as i32;
        let clip_min_y = (clip.min.y * escala) as i32;
        let clip_max_x = (clip.max.x * escala) as i32;
        let clip_max_y = (clip.max.y * escala) as i32;

        for tri in indices.chunks_exact(3) {
            let v0 = &verts[tri[0] as usize];
            let v1 = &verts[tri[1] as usize];
            let v2 = &verts[tri[2] as usize];

            let x0 = v0.pos.x * escala;
            let y0 = v0.pos.y * escala;
            let x1 = v1.pos.x * escala;
            let y1 = v1.pos.y * escala;
            let x2 = v2.pos.x * escala;
            let y2 = v2.pos.y * escala;

            let min_x = x0.min(x1).min(x2).floor() as i32;
            let min_y = y0.min(y1).min(y2).floor() as i32;
            let max_x = x0.max(x1).max(x2).ceil() as i32;
            let max_y = y0.max(y1).max(y2).ceil() as i32;

            let min_x = min_x.max(clip_min_x).max(0);
            let min_y = min_y.max(clip_min_y).max(0);
            let max_x = max_x.min(clip_max_x).min(w - 1);
            let max_y = max_y.min(clip_max_y).min(h - 1);

            if min_x > max_x || min_y > max_y {
                continue;
            }

            let area = aresta(x0, y0, x1, y1, x2, y2);
            if area.abs() < 0.5 {
                continue;
            }
            let inv = 1.0 / area;

            for py in min_y..=max_y {
                let fy = py as f32 + 0.5;
                for px in min_x..=max_x {
                    let fx = px as f32 + 0.5;

                    let w0 = aresta(x1, y1, x2, y2, fx, fy) * inv;
                    let w1 = aresta(x2, y2, x0, y0, fx, fy) * inv;
                    let w2 = 1.0 - w0 - w1;

                    if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                        continue;
                    }

                    let cor = interpolar_cor(v0, v1, v2, w0, w1, w2, textura);
                    let a = cor.a();
                    if a == 0 {
                        continue;
                    }

                    let idx = py as usize * self.largura as usize + px as usize;
                    if a == 255 {
                        self.buffer[idx] = cor_para_rgb(cor);
                    } else {
                        self.buffer[idx] = misturar(self.buffer[idx], cor);
                    }
                }
            }
        }
    }
}

#[inline]
fn aresta(x0: f32, y0: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    (x1 - x0) * (y2 - y0) - (y1 - y0) * (x2 - x0)
}

#[inline]
fn interpolar_cor(
    v0: &Vertex,
    v1: &Vertex,
    v2: &Vertex,
    w0: f32,
    w1: f32,
    w2: f32,
    textura: Option<&Textura>,
) -> Color32 {
    let r = w0 * v0.color.r() as f32 + w1 * v1.color.r() as f32 + w2 * v2.color.r() as f32;
    let g = w0 * v0.color.g() as f32 + w1 * v1.color.g() as f32 + w2 * v2.color.g() as f32;
    let b = w0 * v0.color.b() as f32 + w1 * v1.color.b() as f32 + w2 * v2.color.b() as f32;
    let a = w0 * v0.color.a() as f32 + w1 * v1.color.a() as f32 + w2 * v2.color.a() as f32;

    if let Some(tex) = textura {
        let u = w0 * v0.uv.x + w1 * v1.uv.x + w2 * v2.uv.x;
        let v = w0 * v0.uv.y + w1 * v1.uv.y + w2 * v2.uv.y;
        let tc = tex.amostra(u, v);
        Color32::from_rgba_unmultiplied(
            ((r * tc.r() as f32) / 255.0) as u8,
            ((g * tc.g() as f32) / 255.0) as u8,
            ((b * tc.b() as f32) / 255.0) as u8,
            ((a * tc.a() as f32) / 255.0) as u8,
        )
    } else {
        Color32::from_rgba_unmultiplied(r as u8, g as u8, b as u8, a as u8)
    }
}

#[inline]
fn cor_para_rgb(c: Color32) -> u32 {
    (c.r() as u32) << 16 | (c.g() as u32) << 8 | c.b() as u32
}

#[inline]
fn misturar(dst_packed: u32, src: Color32) -> u32 {
    let sa = src.a() as u32;
    let inv = 255 - sa;
    let dr = (dst_packed >> 16) & 0xFF;
    let dg = (dst_packed >> 8) & 0xFF;
    let db = dst_packed & 0xFF;
    let r = (src.r() as u32 * sa + dr * inv) / 255;
    let g = (src.g() as u32 * sa + dg * inv) / 255;
    let b = (src.b() as u32 * sa + db * inv) / 255;
    r << 16 | g << 8 | b
}
